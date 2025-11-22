use rustache::cache::Cache;
use rustache::client::RustyClient;
use rustache::config;
use rustache::model::{CalendarListEntry, Task};
use rustache::ui::{AppState, Focus, InputMode, draw};

use anyhow::Result;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, MouseEventKind},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{Terminal, backend::CrosstermBackend};
use std::{env, io, time::Duration};
use tokio::sync::mpsc;

enum Action {
    SwitchCalendar(String),
    ToggleTask(usize),
    CreateTask(String),
    EditTask(usize, String),
    EditDescription(usize, String),
    DeleteTask(usize),
    ChangePriority(usize, i8),
    IndentTask(usize),
    OutdentTask(usize),
    Quit,
}

enum AppEvent {
    CalendarsLoaded(Vec<CalendarListEntry>),
    TasksLoaded(Vec<Task>),
    #[allow(dead_code)]
    TaskUpdated(Task),
    Error(String),
    Status(String),
}

#[tokio::main]
async fn main() -> Result<()> {
    let default_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        use std::io::Write;
        if let Ok(mut file) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open("rustache_panic.log")
        {
            let _ = writeln!(file, "PANIC: {:?}", info);
        }
        default_hook(info);
    }));

    let (url, user, pass, default_cal) = match config::Config::load() {
        Ok(cfg) => (cfg.url, cfg.username, cfg.password, cfg.default_calendar),
        Err(_) => {
            let args: Vec<String> = env::args().collect();
            if args.len() < 4 {
                return Ok(());
            }
            (args[1].clone(), args[2].clone(), args[3].clone(), None)
        }
    };

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app_state = AppState::new();
    let (action_tx, mut action_rx) = mpsc::channel(10);
    let (event_tx, mut event_rx) = mpsc::channel(10);

    tokio::spawn(async move {
        let mut client = match RustyClient::new(&url, &user, &pass) {
            Ok(c) => c,
            Err(e) => {
                let _ = event_tx.send(AppEvent::Error(e)).await;
                return;
            }
        };

        let _ = event_tx
            .send(AppEvent::Status("Connecting...".to_string()))
            .await;

        let calendars = match client.get_calendars().await {
            Ok(cals) => {
                let _ = event_tx.send(AppEvent::CalendarsLoaded(cals.clone())).await;
                Some(cals)
            }
            Err(e) => {
                let _ = event_tx
                    .send(AppEvent::Status(format!("Cal warning: {}", e)))
                    .await;
                None
            }
        };

        let mut active_href = None;

        if let Some(def_name) = &default_cal {
            if let Some(cals) = &calendars {
                if let Some(found) = cals
                    .iter()
                    .find(|c| c.name == *def_name || c.href == *def_name)
                {
                    client.set_calendar(&found.href);
                    active_href = Some(found.href.clone());
                }
            }
        }

        if active_href.is_none() {
            if let Ok(href) = client.discover_calendar().await {
                active_href = Some(href);
            }
        }

        if let Some(href) = &active_href {
            if let Ok(cached) = Cache::load(href) {
                let organized = Task::organize_hierarchy(cached);
                let _ = event_tx.send(AppEvent::TasksLoaded(organized)).await;
                let _ = event_tx
                    .send(AppEvent::Status("Loaded from cache.".to_string()))
                    .await;
            }
        }

        let _ = event_tx
            .send(AppEvent::Status("Syncing...".to_string()))
            .await;
        let mut local_tasks: Vec<Task> = match client.get_tasks().await {
            Ok(t) => {
                let organized = Task::organize_hierarchy(t.clone());
                if let Some(href) = &active_href {
                    let _ = Cache::save(href, &t);
                }
                organized
            }
            Err(e) => {
                let _ = event_tx.send(AppEvent::Error(e)).await;
                vec![]
            }
        };
        let _ = event_tx
            .send(AppEvent::TasksLoaded(local_tasks.clone()))
            .await;
        let _ = event_tx.send(AppEvent::Status("Ready.".to_string())).await;

        while let Some(action) = action_rx.recv().await {
            match action {
                Action::Quit => break,

                Action::SwitchCalendar(href) => {
                    active_href = Some(href.clone());
                    let _ = event_tx
                        .send(AppEvent::Status("Switching...".to_string()))
                        .await;

                    if let Ok(cached) = Cache::load(&href) {
                        local_tasks = Task::organize_hierarchy(cached);
                        let _ = event_tx
                            .send(AppEvent::TasksLoaded(local_tasks.clone()))
                            .await;
                    } else {
                        local_tasks.clear();
                        let _ = event_tx.send(AppEvent::TasksLoaded(vec![])).await;
                    }

                    client.set_calendar(&href);
                    match client.get_tasks().await {
                        Ok(t) => {
                            local_tasks = Task::organize_hierarchy(t.clone());
                            if let Some(href) = &active_href {
                                let _ = Cache::save(href, &t);
                            } // Inline save
                            let _ = event_tx
                                .send(AppEvent::TasksLoaded(local_tasks.clone()))
                                .await;
                            let _ = event_tx.send(AppEvent::Status("Ready.".to_string())).await;
                        }
                        Err(e) => {
                            let _ = event_tx.send(AppEvent::Error(e)).await;
                        }
                    }
                }

                Action::CreateTask(summary) => {
                    let _ = event_tx
                        .send(AppEvent::Status("Creating...".to_string()))
                        .await;
                    let mut new_task = Task::new(&summary);
                    match client.create_task(&mut new_task).await {
                        Ok(_) => {
                            local_tasks.push(new_task);
                            local_tasks = Task::organize_hierarchy(local_tasks);
                            if let Some(href) = &active_href {
                                let _ = Cache::save(href, &local_tasks);
                            }
                            let _ = event_tx
                                .send(AppEvent::TasksLoaded(local_tasks.clone()))
                                .await;
                            let _ = event_tx
                                .send(AppEvent::Status("Created.".to_string()))
                                .await;
                        }
                        Err(e) => {
                            let _ = event_tx.send(AppEvent::Error(e)).await;
                        }
                    }
                }

                Action::EditTask(index, smart_string) => {
                    if index < local_tasks.len() {
                        let task = &mut local_tasks[index];
                        task.apply_smart_input(&smart_string);
                        let _ = event_tx
                            .send(AppEvent::Status("Updating...".to_string()))
                            .await;
                        let mut task_copy = task.clone();
                        match client.update_task(&mut task_copy).await {
                            Ok(_) => {
                                local_tasks[index] = task_copy;
                                local_tasks = Task::organize_hierarchy(local_tasks);
                                if let Some(href) = &active_href {
                                    let _ = Cache::save(href, &local_tasks);
                                }
                                let _ = event_tx
                                    .send(AppEvent::TasksLoaded(local_tasks.clone()))
                                    .await;
                                let _ = event_tx
                                    .send(AppEvent::Status("Updated.".to_string()))
                                    .await;
                            }
                            Err(e) => {
                                let _ = event_tx.send(AppEvent::Error(e)).await;
                            }
                        }
                    }
                }

                Action::EditDescription(index, description) => {
                    if index < local_tasks.len() {
                        let task = &mut local_tasks[index];
                        task.description = description;
                        let _ = event_tx
                            .send(AppEvent::Status("Updating Note...".to_string()))
                            .await;
                        let mut task_copy = task.clone();
                        match client.update_task(&mut task_copy).await {
                            Ok(_) => {
                                local_tasks[index] = task_copy;
                                local_tasks = Task::organize_hierarchy(local_tasks);
                                if let Some(href) = &active_href {
                                    let _ = Cache::save(href, &local_tasks);
                                }
                                let _ = event_tx
                                    .send(AppEvent::TasksLoaded(local_tasks.clone()))
                                    .await;
                                let _ = event_tx
                                    .send(AppEvent::Status("Updated.".to_string()))
                                    .await;
                            }
                            Err(e) => {
                                let _ = event_tx.send(AppEvent::Error(e)).await;
                            }
                        }
                    }
                }

                Action::ToggleTask(index) => {
                    if index < local_tasks.len() {
                        let task = &mut local_tasks[index];
                        let _ = event_tx
                            .send(AppEvent::Status("Syncing...".to_string()))
                            .await;
                        let mut task_copy = task.clone();

                        match client.toggle_task(&mut task_copy).await {
                            Ok((updated, created_opt)) => {
                                local_tasks[index] = updated;
                                if let Some(created) = created_opt {
                                    local_tasks.push(created);
                                }
                                local_tasks = Task::organize_hierarchy(local_tasks);
                                if let Some(href) = &active_href {
                                    let _ = Cache::save(href, &local_tasks);
                                }
                                let _ = event_tx
                                    .send(AppEvent::TasksLoaded(local_tasks.clone()))
                                    .await;
                                let _ =
                                    event_tx.send(AppEvent::Status("Synced.".to_string())).await;
                            }
                            Err(e) => {
                                let _ = event_tx
                                    .send(AppEvent::Error(format!("Sync Failed: {}", e)))
                                    .await;
                            }
                        }
                    }
                }

                Action::DeleteTask(index) => {
                    if index < local_tasks.len() {
                        let task = local_tasks[index].clone();
                        let _ = event_tx
                            .send(AppEvent::Status("Deleting...".to_string()))
                            .await;
                        match client.delete_task(&task).await {
                            Ok(_) => {
                                local_tasks.remove(index);
                                local_tasks = Task::organize_hierarchy(local_tasks);
                                if let Some(href) = &active_href {
                                    let _ = Cache::save(href, &local_tasks);
                                }
                                let _ = event_tx
                                    .send(AppEvent::TasksLoaded(local_tasks.clone()))
                                    .await;
                                let _ = event_tx
                                    .send(AppEvent::Status("Deleted.".to_string()))
                                    .await;
                            }
                            Err(e) => {
                                let _ = event_tx.send(AppEvent::Error(e)).await;
                            }
                        }
                    }
                }

                Action::ChangePriority(index, delta) => {
                    if index < local_tasks.len() {
                        let task = &mut local_tasks[index];
                        let new_prio = if delta > 0 {
                            match task.priority {
                                0 => 9,
                                9 => 5,
                                5 => 1,
                                1 => 1,
                                _ => 5,
                            }
                        } else {
                            match task.priority {
                                1 => 5,
                                5 => 9,
                                9 => 0,
                                0 => 0,
                                _ => 0,
                            }
                        };
                        if new_prio != task.priority {
                            task.priority = new_prio;
                            let _ = event_tx
                                .send(AppEvent::Status("Updating Prio...".to_string()))
                                .await;
                            let mut task_copy = task.clone();
                            match client.update_task(&mut task_copy).await {
                                Ok(_) => {
                                    local_tasks[index] = task_copy;
                                    local_tasks = Task::organize_hierarchy(local_tasks);
                                    if let Some(href) = &active_href {
                                        let _ = Cache::save(href, &local_tasks);
                                    }
                                    let _ = event_tx
                                        .send(AppEvent::TasksLoaded(local_tasks.clone()))
                                        .await;
                                    let _ = event_tx
                                        .send(AppEvent::Status("Updated.".to_string()))
                                        .await;
                                }
                                Err(e) => {
                                    let _ = event_tx.send(AppEvent::Error(e)).await;
                                }
                            }
                        }
                    }
                }

                Action::IndentTask(index) => {
                    if index > 0 && index < local_tasks.len() {
                        let parent_candidate = local_tasks[index - 1].uid.clone();
                        if local_tasks[index].parent_uid != Some(parent_candidate.clone()) {
                            let task = &mut local_tasks[index];
                            task.parent_uid = Some(parent_candidate);
                            let _ = event_tx
                                .send(AppEvent::Status("Indenting...".to_string()))
                                .await;
                            let mut task_copy = task.clone();
                            match client.update_task(&mut task_copy).await {
                                Ok(_) => {
                                    local_tasks[index] = task_copy;
                                    local_tasks = Task::organize_hierarchy(local_tasks);
                                    if let Some(href) = &active_href {
                                        let _ = Cache::save(href, &local_tasks);
                                    }
                                    let _ = event_tx
                                        .send(AppEvent::TasksLoaded(local_tasks.clone()))
                                        .await;
                                    let _ = event_tx
                                        .send(AppEvent::Status("Indented.".to_string()))
                                        .await;
                                }
                                Err(e) => {
                                    let _ = event_tx.send(AppEvent::Error(e)).await;
                                }
                            }
                        }
                    }
                }

                Action::OutdentTask(index) => {
                    if index < local_tasks.len() {
                        let task = &mut local_tasks[index];
                        if task.parent_uid.is_some() {
                            task.parent_uid = None;
                            let _ = event_tx
                                .send(AppEvent::Status("Outdenting...".to_string()))
                                .await;
                            let mut task_copy = task.clone();
                            match client.update_task(&mut task_copy).await {
                                Ok(_) => {
                                    local_tasks[index] = task_copy;
                                    local_tasks = Task::organize_hierarchy(local_tasks);
                                    if let Some(href) = &active_href {
                                        let _ = Cache::save(href, &local_tasks);
                                    }
                                    let _ = event_tx
                                        .send(AppEvent::TasksLoaded(local_tasks.clone()))
                                        .await;
                                    let _ = event_tx
                                        .send(AppEvent::Status("Outdented.".to_string()))
                                        .await;
                                }
                                Err(e) => {
                                    let _ = event_tx.send(AppEvent::Error(e)).await;
                                }
                            }
                        }
                    }
                }
            }
        }
    });

    loop {
        terminal.draw(|f| draw(f, &mut app_state))?;

        if let Ok(event) = event_rx.try_recv() {
            match event {
                AppEvent::CalendarsLoaded(cals) => {
                    app_state.calendars = cals;
                }
                AppEvent::TasksLoaded(tasks) => {
                    app_state.tasks = tasks;
                    app_state.recalculate_view();
                    app_state.loading = false;
                    if app_state.tasks.is_empty() {
                        app_state.message = "No tasks found.".to_string();
                    } else {
                        app_state.message = format!("Tasks: {}", app_state.tasks.len());
                    }
                }
                AppEvent::TaskUpdated(_) => {}
                AppEvent::Error(msg) => {
                    app_state.message = format!("Error: {}", msg);
                    app_state.loading = false;
                }
                AppEvent::Status(msg) => {
                    app_state.message = msg;
                }
            }
        }

        if crossterm::event::poll(Duration::from_millis(50))? {
            let event = event::read()?;
            match event {
                Event::Mouse(mouse) => match mouse.kind {
                    MouseEventKind::ScrollDown => app_state.next(),
                    MouseEventKind::ScrollUp => app_state.previous(),
                    _ => {}
                },
                Event::Key(key) => match app_state.mode {
                    InputMode::Creating => match key.code {
                        KeyCode::Enter => {
                            if !app_state.input_buffer.is_empty() {
                                let summary = app_state.input_buffer.clone();
                                let _ = action_tx.send(Action::CreateTask(summary)).await;
                                app_state.input_buffer.clear();
                                app_state.mode = InputMode::Normal;
                            }
                        }
                        KeyCode::Esc => {
                            app_state.mode = InputMode::Normal;
                            app_state.input_buffer.clear();
                        }
                        KeyCode::Char(c) => app_state.enter_char(c),
                        KeyCode::Backspace => app_state.delete_char(),
                        _ => {}
                    },

                    InputMode::Editing => match key.code {
                        KeyCode::Enter => {
                            if let Some(idx) = app_state.editing_index {
                                let new_text = app_state.input_buffer.clone();
                                let _ = action_tx.send(Action::EditTask(idx, new_text)).await;
                            }
                            app_state.input_buffer.clear();
                            app_state.editing_index = None;
                            app_state.mode = InputMode::Normal;
                        }
                        KeyCode::Esc => {
                            app_state.mode = InputMode::Normal;
                            app_state.input_buffer.clear();
                            app_state.editing_index = None;
                        }
                        KeyCode::Char(c) => app_state.enter_char(c),
                        KeyCode::Backspace => app_state.delete_char(),
                        KeyCode::Left => app_state.move_cursor_left(),
                        KeyCode::Right => app_state.move_cursor_right(),
                        _ => {}
                    },

                    InputMode::EditingDescription => match key.code {
                        KeyCode::Enter => {
                            if let Some(idx) = app_state.editing_index {
                                let new_desc = app_state.input_buffer.clone();
                                let _ =
                                    action_tx.send(Action::EditDescription(idx, new_desc)).await;
                            }
                            app_state.reset_input();
                            app_state.editing_index = None;
                            app_state.mode = InputMode::Normal;
                        }
                        KeyCode::Esc => {
                            app_state.mode = InputMode::Normal;
                            app_state.reset_input();
                            app_state.editing_index = None;
                        }
                        KeyCode::Char(c) => app_state.enter_char(c),
                        KeyCode::Backspace => app_state.delete_char(),
                        KeyCode::Left => app_state.move_cursor_left(),
                        KeyCode::Right => app_state.move_cursor_right(),
                        _ => {}
                    },

                    InputMode::Searching => match key.code {
                        KeyCode::Enter | KeyCode::Esc => {
                            app_state.mode = InputMode::Normal;
                        }
                        KeyCode::Left => app_state.move_cursor_left(),
                        KeyCode::Right => app_state.move_cursor_right(),
                        KeyCode::Char(c) => {
                            app_state.enter_char(c);
                            app_state.recalculate_view();
                        }
                        KeyCode::Backspace => {
                            app_state.delete_char();
                            app_state.recalculate_view();
                        }
                        _ => {}
                    },

                    InputMode::Normal => match key.code {
                        KeyCode::Char('q') => {
                            let _ = action_tx.send(Action::Quit).await;
                            break;
                        }

                        KeyCode::Char('/') => {
                            app_state.mode = InputMode::Searching;
                            app_state.reset_input();
                            app_state.recalculate_view();
                        }
                        KeyCode::Esc => {
                            app_state.reset_input();
                            app_state.recalculate_view();
                        }
                        KeyCode::Tab => app_state.toggle_focus(),
                        KeyCode::Enter => {
                            if app_state.active_focus == Focus::Sidebar {
                                if let Some(idx) = app_state.cal_state.selected() {
                                    if idx < app_state.calendars.len() {
                                        let href = app_state.calendars[idx].href.clone();
                                        let _ = action_tx.send(Action::SwitchCalendar(href)).await;
                                        app_state.active_focus = Focus::Main;
                                    }
                                }
                            }
                        }
                        KeyCode::Char('a') => {
                            app_state.mode = InputMode::Creating;
                            app_state.reset_input();
                            app_state.message = "Example: Buy Milk @tomorrow !1".to_string();
                        }
                        KeyCode::Char('e') => {
                            if app_state.active_focus == Focus::Main {
                                if let Some(idx) = app_state.get_selected_master_index() {
                                    let task = &app_state.tasks[idx];
                                    app_state.mode = InputMode::Editing;
                                    let text = task.to_smart_string();
                                    app_state.input_buffer = text.clone();
                                    app_state.cursor_position = text.chars().count();
                                    app_state.editing_index = Some(idx);
                                }
                            }
                        }
                        KeyCode::Char('E') => {
                            if app_state.active_focus == Focus::Main {
                                if let Some(idx) = app_state.get_selected_master_index() {
                                    let task = &app_state.tasks[idx];
                                    app_state.mode = InputMode::EditingDescription;
                                    let text = task.description.clone();
                                    app_state.input_buffer = text.clone();
                                    app_state.cursor_position = text.chars().count();
                                    app_state.editing_index = Some(idx);
                                }
                            }
                        }
                        KeyCode::Down | KeyCode::Char('j') => app_state.next(),
                        KeyCode::Up | KeyCode::Char('k') => app_state.previous(),
                        KeyCode::PageDown => app_state.jump_forward(10),
                        KeyCode::PageUp => app_state.jump_backward(10),
                        KeyCode::Char(' ') => {
                            if app_state.active_focus == Focus::Main {
                                if let Some(idx) = app_state.get_selected_master_index() {
                                    let _ = action_tx.send(Action::ToggleTask(idx)).await;
                                }
                            }
                        }
                        KeyCode::Char('d') => {
                            if app_state.active_focus == Focus::Main {
                                if let Some(idx) = app_state.get_selected_master_index() {
                                    let _ = action_tx.send(Action::DeleteTask(idx)).await;
                                }
                            }
                        }
                        KeyCode::Char('+') => {
                            if app_state.active_focus == Focus::Main {
                                if let Some(idx) = app_state.get_selected_master_index() {
                                    let _ = action_tx.send(Action::ChangePriority(idx, 1)).await;
                                }
                            }
                        }
                        KeyCode::Char('-') => {
                            if app_state.active_focus == Focus::Main {
                                if let Some(idx) = app_state.get_selected_master_index() {
                                    let _ = action_tx.send(Action::ChangePriority(idx, -1)).await;
                                }
                            }
                        }
                        KeyCode::Char('.') | KeyCode::Char('>') => {
                            if app_state.active_focus == Focus::Main {
                                if let Some(idx) = app_state.get_selected_master_index() {
                                    let _ = action_tx.send(Action::IndentTask(idx)).await;
                                }
                            }
                        }
                        KeyCode::Char(',') | KeyCode::Char('<') => {
                            if app_state.active_focus == Focus::Main {
                                if let Some(idx) = app_state.get_selected_master_index() {
                                    let _ = action_tx.send(Action::OutdentTask(idx)).await;
                                }
                            }
                        }
                        _ => {}
                    },
                },
                _ => {}
            }
        }
    }

    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    Ok(())
}
