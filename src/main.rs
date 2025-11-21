mod client;
mod config;
mod model;
mod ui;

use crate::client::RustyClient;
use crate::model::{CalendarListEntry, Task};
use crate::ui::{AppState, Focus, draw};
use anyhow::Result;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, MouseEventKind},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{Terminal, backend::CrosstermBackend};
use std::{env, io, time::Duration};
use tokio::sync::mpsc;

// 1. Actions (UI -> Network)
enum Action {
    SwitchCalendar(String), // Href
    ToggleTask(usize),
    CreateTask(String),
    DeleteTask(usize),
    ChangePriority(usize, i8),
    Quit,
}

// 2. Events (Network -> UI)
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
    // 0. Panic Hook (Logs crashes to file)
    let default_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        use std::io::Write;
        if let Ok(mut file) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open("rustycal_panic.log")
        {
            let _ = writeln!(file, "PANIC: {:?}", info);
        }
        default_hook(info);
    }));

    // 1. Load Config
    let (url, user, pass, default_cal) = match config::Config::load() {
        Ok(cfg) => (cfg.url, cfg.username, cfg.password, cfg.default_calendar),
        Err(_) => {
            // Fallback (default_cal is None)
            let args: Vec<String> = env::args().collect();
            if args.len() < 4 {
                /* ... error ... */
                return Ok(());
            }
            (args[1].clone(), args[2].clone(), args[3].clone(), None)
        }
    };

    // 2. Setup Terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // 3. Setup State & Channels
    let mut app_state = AppState::new();
    let (action_tx, mut action_rx) = mpsc::channel(10);
    let (event_tx, mut event_rx) = mpsc::channel(10);

    // 4. Spawn Background Actor
    // Spawn Background Actor
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

        // A. Fetch Calendars List
        let calendars = match client.get_calendars().await {
            Ok(cals) => {
                let _ = event_tx.send(AppEvent::CalendarsLoaded(cals.clone())).await;
                Some(cals)
            }
            Err(e) => {
                let _ = event_tx
                    .send(AppEvent::Status(format!("Cal discovery warning: {}", e)))
                    .await;
                None
            }
        };

        // B. Select Active Calendar
        // Logic:
        // 1. If config has `default_calendar`, try to match it by Name or URL.
        // 2. If not found/set, fall back to `discover_calendar` (Automatic/First).

        let mut selected = false;

        if let Some(def_name) = &default_cal {
            if let Some(cals) = &calendars {
                // Try to find matching calendar by Name (Display Name) or Href
                if let Some(found) = cals
                    .iter()
                    .find(|c| c.name == *def_name || c.href == *def_name)
                {
                    let _ = event_tx
                        .send(AppEvent::Status(format!("Opening '{}'...", found.name)))
                        .await;
                    client.set_calendar(&found.href);
                    selected = true;
                }
            }
        }

        if !selected {
            if let Err(e) = client.discover_calendar().await {
                let _ = event_tx.send(AppEvent::Error(e)).await;
                return;
            }
        }

        let _ = event_tx
            .send(AppEvent::Status("Fetching tasks...".to_string()))
            .await;

        // C. Initial Task Fetch
        let mut local_tasks: Vec<Task> = match client.get_tasks().await {
            Ok(t) => t,
            Err(e) => {
                let _ = event_tx.send(AppEvent::Error(e)).await;
                return;
            }
        };

        local_tasks.sort();
        let _ = event_tx
            .send(AppEvent::TasksLoaded(local_tasks.clone()))
            .await;

        // ... (The rest of the loop remains exactly the same) ...
        // D. Action Loop
        while let Some(action) = action_rx.recv().await {
            match action {
                Action::Quit => break,

                // --- SWITCH CALENDAR ---
                Action::SwitchCalendar(href) => {
                    let _ = event_tx
                        .send(AppEvent::Status("Switching...".to_string()))
                        .await;
                    client.set_calendar(&href);

                    match client.get_tasks().await {
                        Ok(t) => {
                            local_tasks = t;
                            local_tasks.sort();
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

                // --- CREATE ---
                Action::CreateTask(summary) => {
                    let _ = event_tx
                        .send(AppEvent::Status("Creating...".to_string()))
                        .await;
                    let mut new_task = Task::new(&summary);

                    match client.create_task(&mut new_task).await {
                        Ok(_) => {
                            local_tasks.push(new_task);
                            local_tasks.sort();
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

                // --- TOGGLE ---
                Action::ToggleTask(index) => {
                    if index < local_tasks.len() {
                        let task = &mut local_tasks[index];
                        task.completed = !task.completed;
                        let _ = event_tx
                            .send(AppEvent::Status("Syncing...".to_string()))
                            .await;

                        let mut task_copy = task.clone();
                        match client.update_task(&mut task_copy).await {
                            Ok(_) => {
                                local_tasks[index] = task_copy.clone();
                                local_tasks.sort();
                                let _ = event_tx
                                    .send(AppEvent::TasksLoaded(local_tasks.clone()))
                                    .await;
                                let _ =
                                    event_tx.send(AppEvent::Status("Synced.".to_string())).await;
                            }
                            Err(e) => {
                                local_tasks[index].completed = !local_tasks[index].completed;
                                let _ = event_tx
                                    .send(AppEvent::Error(format!("Sync Failed: {}", e)))
                                    .await;
                            }
                        }
                    }
                }

                // --- DELETE ---
                Action::DeleteTask(index) => {
                    if index < local_tasks.len() {
                        let task = local_tasks[index].clone();
                        let _ = event_tx
                            .send(AppEvent::Status("Deleting...".to_string()))
                            .await;

                        match client.delete_task(&task).await {
                            Ok(_) => {
                                local_tasks.remove(index);
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

                // --- PRIORITY ---
                Action::ChangePriority(index, delta) => {
                    if index < local_tasks.len() {
                        let task = &mut local_tasks[index];

                        // 1 (High) <-> 5 (Med) <-> 9 (Low) <-> 0 (None)
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
                                    local_tasks.sort();
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
            }
        }
    });

    // 5. UI Loop
    loop {
        terminal.draw(|f| draw(f, &mut app_state))?;

        // Process Network Events
        if let Ok(event) = event_rx.try_recv() {
            match event {
                AppEvent::CalendarsLoaded(cals) => {
                    app_state.calendars = cals;
                }
                AppEvent::TasksLoaded(tasks) => {
                    app_state.tasks = tasks;
                    app_state.loading = false;
                    // Update message based on focus
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

        // Process User Input
        if crossterm::event::poll(Duration::from_millis(50))? {
            let event = event::read()?;

            match event {
                Event::Mouse(mouse) => match mouse.kind {
                    MouseEventKind::ScrollDown => app_state.next(),
                    MouseEventKind::ScrollUp => app_state.previous(),
                    _ => {}
                },
                Event::Key(key) => {
                    if app_state.show_input {
                        // --- INPUT MODE ---
                        match key.code {
                            KeyCode::Enter => {
                                if !app_state.input_buffer.is_empty() {
                                    let summary = app_state.input_buffer.clone();
                                    let _ = action_tx.send(Action::CreateTask(summary)).await;
                                    app_state.input_buffer.clear();
                                    app_state.show_input = false;
                                }
                            }
                            KeyCode::Esc => {
                                app_state.show_input = false;
                                app_state.input_buffer.clear();
                            }
                            KeyCode::Char(c) => app_state.input_buffer.push(c),
                            KeyCode::Backspace => {
                                app_state.input_buffer.pop();
                            }
                            _ => {}
                        }
                    } else {
                        // --- NORMAL MODE ---
                        match key.code {
                            KeyCode::Char('q') => {
                                let _ = action_tx.send(Action::Quit).await;
                                break;
                            }
                            KeyCode::Tab => {
                                app_state.toggle_focus();
                            }
                            KeyCode::Enter => {
                                // If Sidebar Focused -> Switch Calendar
                                if app_state.active_focus == Focus::Sidebar {
                                    if let Some(idx) = app_state.cal_state.selected() {
                                        if idx < app_state.calendars.len() {
                                            let href = app_state.calendars[idx].href.clone();
                                            let _ =
                                                action_tx.send(Action::SwitchCalendar(href)).await;
                                            // Auto switch back to main view
                                            app_state.active_focus = Focus::Main;
                                        }
                                    }
                                }
                            }
                            KeyCode::Char('a') => {
                                app_state.show_input = true;
                                app_state.message = "Example: Buy Milk @tomorrow !1".to_string();
                            }
                            // Navigation
                            KeyCode::Down | KeyCode::Char('j') => app_state.next(),
                            KeyCode::Up | KeyCode::Char('k') => app_state.previous(),
                            KeyCode::PageDown => app_state.jump_forward(10),
                            KeyCode::PageUp => app_state.jump_backward(10),

                            // Task Actions (Only if Main Focus)
                            KeyCode::Char(' ') => {
                                if app_state.active_focus == Focus::Main {
                                    if let Some(idx) = app_state.list_state.selected() {
                                        if idx < app_state.tasks.len() {
                                            // Optimistic Toggle
                                            app_state.tasks[idx].completed =
                                                !app_state.tasks[idx].completed;
                                            let _ = action_tx.send(Action::ToggleTask(idx)).await;
                                        }
                                    }
                                }
                            }
                            KeyCode::Char('d') => {
                                if app_state.active_focus == Focus::Main {
                                    if let Some(idx) = app_state.list_state.selected() {
                                        if idx < app_state.tasks.len() {
                                            let _ = action_tx.send(Action::DeleteTask(idx)).await;
                                        }
                                    }
                                }
                            }
                            KeyCode::Char('+') => {
                                if app_state.active_focus == Focus::Main {
                                    if let Some(idx) = app_state.list_state.selected() {
                                        if idx < app_state.tasks.len() {
                                            let _ = action_tx
                                                .send(Action::ChangePriority(idx, 1))
                                                .await;
                                        }
                                    }
                                }
                            }
                            KeyCode::Char('-') => {
                                if app_state.active_focus == Focus::Main {
                                    if let Some(idx) = app_state.list_state.selected() {
                                        if idx < app_state.tasks.len() {
                                            let _ = action_tx
                                                .send(Action::ChangePriority(idx, -1))
                                                .await;
                                        }
                                    }
                                }
                            }
                            _ => {}
                        }
                    }
                }
                _ => {}
            }
        }
    }

    // Cleanup
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    Ok(())
}
