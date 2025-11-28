pub mod action;
pub mod state;
pub mod view;

use crate::cache::Cache;
use crate::client::RustyClient;
use crate::config;
use crate::model::{CalendarListEntry, Task};
use crate::storage::{LOCAL_CALENDAR_HREF, LOCAL_CALENDAR_NAME, LocalStorage};
use action::{Action, AppEvent, SidebarMode};
use state::{AppState, Focus, InputMode};
use view::draw;

use anyhow::Result;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, MouseEventKind},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{Terminal, backend::CrosstermBackend};
use std::{env, io, time::Duration};
use tokio::sync::mpsc;

pub async fn run() -> Result<()> {
    // --- 1. PREAMBLE & CONFIG ---
    let args: Vec<String> = env::args().collect();
    if args.len() > 1 && (args[1] == "--help" || args[1] == "-h") {
        println!("Usage: cfait [OPTIONS]");
        return Ok(());
    }

    let default_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        use std::io::Write;
        if let Ok(mut file) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open("cfait_panic.log")
        {
            let _ = writeln!(file, "PANIC: {:?}", info);
        }
        default_hook(info);
    }));

    // Load Config
    let config_result = config::Config::load();
    let (
        url,
        user,
        pass,
        default_cal,
        hide_completed,
        hide_fully_completed_tags,
        tag_aliases,
        sort_cutoff,
        allow_insecure,
        hidden_calendars, // <--- This is the 10th element
    ) = match config_result {
        Ok(cfg) => (
            cfg.url,
            cfg.username,
            cfg.password,
            cfg.default_calendar,
            cfg.hide_completed,
            cfg.hide_fully_completed_tags,
            cfg.tag_aliases,
            cfg.sort_cutoff_months,
            cfg.allow_insecure_certs,
            cfg.hidden_calendars, // <--- This matches
        ),
        Err(_) => {
            let path_str = match config::Config::get_path_string() {
                Ok(path) => path,
                Err(_) => "[Could not determine config path]".to_string(),
            };
            eprintln!("Config file not found.");
            eprintln!("Please create a configuration file at:");
            eprintln!("  {}", path_str);
            eprintln!("\nOr run 'cfait-gui' once to generate it automatically.");
            return Ok(());
        }
    };

    // --- 2. TERMINAL SETUP ---
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // --- 3. STATE INIT ---
    let mut app_state = AppState::new();
    app_state.hide_completed = hide_completed;
    app_state.hide_fully_completed_tags = hide_fully_completed_tags;
    app_state.tag_aliases = tag_aliases;
    app_state.sort_cutoff_months = sort_cutoff;
    app_state.hidden_calendars = hidden_calendars.into_iter().collect();

    let (action_tx, mut action_rx) = mpsc::channel(10);
    let (event_tx, mut event_rx) = mpsc::channel(10);

    // --- NETWORK THREAD ---
    tokio::spawn(async move {
        // Create Client (Unified Repository)
        // If credentials are empty, RustyClient creates an offline client automatically.
        let client = match RustyClient::new(&url, &user, &pass, allow_insecure) {
            Ok(c) => c,
            Err(e) => {
                let _ = event_tx.send(AppEvent::Error(e)).await;
                return;
            }
        };
        let _ = event_tx
            .send(AppEvent::Status("Connecting...".to_string()))
            .await;

        // A. Fetch Calendars (Network)
        // This will be empty if offline or if network fails non-fatally
        let mut calendars = match client.get_calendars().await {
            Ok(cals) => cals,
            Err(e) => {
                let err_str = e.to_string();
                if err_str.contains("InvalidCertificate") {
                    // This is a FATAL configuration error. Build the helpful message and stop.
                    let mut helpful_msg =
                        "Connection failed: The server presented an invalid TLS/SSL certificate."
                            .to_string();

                    // --- START OF MISSING CODE ---
                    let config_advice = format!(
                        "\n\nTo fix this, please edit your config file:\n  {}",
                        crate::config::Config::get_path_string()
                            .unwrap_or_else(|_| "path unknown".to_string())
                    );

                    if !allow_insecure {
                        helpful_msg.push_str(
                            "\nIf this is a self-hosted server (like Radicale), try setting 'allow_insecure_certs = true' in your config.",
                        );
                    } else {
                        helpful_msg.push_str(
                            "\nEven with 'allow_insecure_certs = true', the certificate is invalid. Please check your server's TLS/SSL configuration.",
                        );
                    }
                    helpful_msg.push_str(&config_advice);
                    helpful_msg.push_str(&format!("\n\nDetails: {}", err_str));

                    let _ = event_tx.send(AppEvent::Error(helpful_msg)).await;
                    return;
                    // --- END OF MISSING CODE ---
                } else {
                    // This is a non-fatal network error (e.g., timeout, DNS fail).
                    // We can proceed in offline mode. Send a status update and return an empty list.
                    let _ = event_tx
                        .send(AppEvent::Status(format!("Sync warning: {}", err_str)))
                        .await;
                    vec![]
                }
            }
        };

        // B. Inject Local Calendar
        let local_cal = CalendarListEntry {
            name: LOCAL_CALENDAR_NAME.to_string(),
            href: LOCAL_CALENDAR_HREF.to_string(),
            color: None,
        };
        calendars.push(local_cal);

        let _ = event_tx
            .send(AppEvent::CalendarsLoaded(calendars.clone()))
            .await;

        // C. Fetch All Tasks (Smart Fetch)
        // 1. Load Disk Cache for Network Cals (Performance)
        let mut cached_results = Vec::new();
        for cal in &calendars {
            if cal.href != LOCAL_CALENDAR_HREF {
                if let Ok(tasks) = Cache::load(&cal.href) {
                    cached_results.push((cal.href.clone(), tasks));
                }
            }
        }
        if !cached_results.is_empty() {
            let _ = event_tx.send(AppEvent::TasksLoaded(cached_results)).await;
            let _ = event_tx
                .send(AppEvent::Status("Loaded cache.".to_string()))
                .await;
        }

        // 2. Fetch Fresh Data (Network + Local Disk)
        // The RustyClient::get_all_tasks now handles routing!
        let _ = event_tx
            .send(AppEvent::Status("Syncing...".to_string()))
            .await;
        match client.get_all_tasks(&calendars).await {
            Ok(results) => {
                let _ = event_tx.send(AppEvent::TasksLoaded(results)).await;
                let _ = event_tx.send(AppEvent::Status("Ready.".to_string())).await;
            }
            Err(e) => {
                let _ = event_tx
                    .send(AppEvent::Status(format!("Sync warning: {}", e)))
                    .await;
            }
        }

        // C. Action Loop
        while let Some(action) = action_rx.recv().await {
            match action {
                Action::Quit => break,

                Action::SwitchCalendar(href) => match client.get_tasks(&href).await {
                    Ok(t) => {
                        let _ = event_tx.send(AppEvent::TasksLoaded(vec![(href, t)])).await;
                    }
                    Err(e) => {
                        let _ = event_tx.send(AppEvent::Error(e)).await;
                    }
                },

                Action::CreateTask(mut new_task) => {
                    let href = new_task.calendar_href.clone();
                    // We use the task passed in, we do NOT call Task::new here
                    match client.create_task(&mut new_task).await {
                        Ok(_) => {
                            if let Ok(t) = client.get_tasks(&href).await {
                                let _ = event_tx.send(AppEvent::TasksLoaded(vec![(href, t)])).await;
                            }
                            let _ = event_tx
                                .send(AppEvent::Status("Created.".to_string()))
                                .await;
                        }
                        Err(e) => {
                            let _ = event_tx.send(AppEvent::Error(e)).await;
                        }
                    }
                }

                Action::UpdateTask(mut task) => {
                    let href = task.calendar_href.clone();
                    match client.update_task(&mut task).await {
                        Ok(_) => {
                            let _ = event_tx.send(AppEvent::Status("Saved.".to_string())).await;
                        }
                        Err(e) => {
                            let _ = event_tx
                                .send(AppEvent::Error(format!(
                                    "Sync failed (refreshing...): {}",
                                    e
                                )))
                                .await;
                            // Auto-heal: fetch latest state
                            if let Ok(t) = client.get_tasks(&href).await {
                                let _ = event_tx.send(AppEvent::TasksLoaded(vec![(href, t)])).await;
                            }
                        }
                    }
                }

                Action::ToggleTask(mut task) => {
                    let href = task.calendar_href.clone();
                    // Revert optimistic flip for API logic
                    // (Actually the API logic I gave you uses 'toggle_task' which flips it AGAIN.
                    // So we must pass the state BEFORE the optimistic flip).

                    // Correct approach: We passed the FLIPPED task in the action.
                    // We need to revert it to the OLD state, so the Client method can flip it back to NEW state
                    // and respawn if needed.
                    if task.status == crate::model::TaskStatus::Completed {
                        task.status = crate::model::TaskStatus::NeedsAction;
                    } else {
                        task.status = crate::model::TaskStatus::Completed;
                    }

                    match client.toggle_task(&mut task).await {
                        Ok(_) => {
                            let _ = event_tx.send(AppEvent::Status("Synced.".to_string())).await;
                            if let Ok(t) = client.get_tasks(&href).await {
                                let _ = event_tx.send(AppEvent::TasksLoaded(vec![(href, t)])).await;
                            }
                        }
                        Err(e) => {
                            let _ = event_tx
                                .send(AppEvent::Error(format!(
                                    "Toggle failed (refreshing...): {}",
                                    e
                                )))
                                .await;
                            if let Ok(t) = client.get_tasks(&href).await {
                                let _ = event_tx.send(AppEvent::TasksLoaded(vec![(href, t)])).await;
                            }
                        }
                    }
                }

                Action::DeleteTask(task) => {
                    let _href = task.calendar_href.clone();
                    match client.delete_task(&task).await {
                        Ok(_) => {
                            let _ = event_tx
                                .send(AppEvent::Status("Deleted.".to_string()))
                                .await;
                        }
                        Err(e) => {
                            let href = task.calendar_href.clone();
                            let _ = event_tx
                                .send(AppEvent::Error(format!(
                                    "Delete failed (refreshing...): {}",
                                    e
                                )))
                                .await;
                            if let Ok(t) = client.get_tasks(&href).await {
                                let _ = event_tx.send(AppEvent::TasksLoaded(vec![(href, t)])).await;
                            }
                        }
                    }
                }
                Action::Refresh => {
                    let _ = event_tx
                        .send(AppEvent::Status("Refreshing...".to_string()))
                        .await;

                    // 1. Fetch Network Calendars
                    let mut calendars = match client.get_calendars().await {
                        Ok(c) => c,
                        Err(e) => {
                            let _ = event_tx.send(AppEvent::Error(e)).await;
                            vec![] // Return empty on error so we can still inject Local
                        }
                    };

                    // 2. INJECT LOCAL CALENDAR
                    let local_cal = CalendarListEntry {
                        name: LOCAL_CALENDAR_NAME.to_string(),
                        href: LOCAL_CALENDAR_HREF.to_string(),
                        color: None,
                    };
                    calendars.push(local_cal);
                    // ----------------------------------------

                    let _ = event_tx
                        .send(AppEvent::CalendarsLoaded(calendars.clone()))
                        .await;

                    // 3. Fetch Tasks (Using the unified client)
                    match client.get_all_tasks(&calendars).await {
                        Ok(results) => {
                            let _ = event_tx.send(AppEvent::TasksLoaded(results)).await;
                            let _ = event_tx
                                .send(AppEvent::Status("Refreshed.".to_string()))
                                .await;
                        }
                        Err(e) => {
                            let _ = event_tx.send(AppEvent::Error(e)).await;
                        }
                    }
                }
                Action::MarkInProcess(mut task) => {
                    if task.status == crate::model::TaskStatus::InProcess {
                        task.status = crate::model::TaskStatus::NeedsAction;
                    } else {
                        task.status = crate::model::TaskStatus::InProcess;
                    }

                    // Direct update instead of using channel
                    match client.update_task(&mut task).await {
                        Ok(_) => {
                            let _ = event_tx.send(AppEvent::Status("Saved.".to_string())).await;
                        }
                        Err(e) => {
                            let _ = event_tx.send(AppEvent::Error(e)).await;
                        }
                    }
                }
                Action::MarkCancelled(mut task) => {
                    if task.status == crate::model::TaskStatus::Cancelled {
                        task.status = crate::model::TaskStatus::NeedsAction;
                    } else {
                        task.status = crate::model::TaskStatus::Cancelled;
                    }

                    match client.update_task(&mut task).await {
                        Ok(_) => {
                            let _ = event_tx.send(AppEvent::Status("Saved.".to_string())).await;
                        }
                        Err(e) => {
                            let _ = event_tx.send(AppEvent::Error(e)).await;
                        }
                    }
                }
                Action::MoveTask(task, new_href) => {
                    let old_href = task.calendar_href.clone();
                    match client.move_task(&task, &new_href).await {
                        Ok(_) => {
                            let _ = event_tx.send(AppEvent::Status("Moved.".to_string())).await;
                            // Refresh both calendars involved
                            // (In a real implementation we might just return the two modified lists,
                            // but fetching them ensures consistency)
                            if let Ok(t1) = client.get_tasks(&old_href).await {
                                let _ = event_tx
                                    .send(AppEvent::TasksLoaded(vec![(old_href, t1)]))
                                    .await;
                            }
                            if let Ok(t2) = client.get_tasks(&new_href).await {
                                let _ = event_tx
                                    .send(AppEvent::TasksLoaded(vec![(new_href, t2)]))
                                    .await;
                            }
                        }
                        Err(e) => {
                            let _ = event_tx
                                .send(AppEvent::Error(format!("Move failed: {}", e)))
                                .await;
                        }
                    }
                }
                Action::MigrateLocal(target_href) => {
                    // Fetch local tasks
                    if let Ok(local_tasks) = LocalStorage::load() {
                        let _ = event_tx
                            .send(AppEvent::Status(format!(
                                "Exporting {} tasks...",
                                local_tasks.len()
                            )))
                            .await;

                        // Use client logic
                        match client.migrate_tasks(local_tasks, &target_href).await {
                            Ok(count) => {
                                let _ = event_tx
                                    .send(AppEvent::Status(format!("Exported {} tasks.", count)))
                                    .await;
                                // Refresh logic: Fetch fresh states for Local (should be empty) and Target
                                if let Ok(t1) = client.get_tasks(LOCAL_CALENDAR_HREF).await {
                                    let _ = event_tx
                                        .send(AppEvent::TasksLoaded(vec![(
                                            LOCAL_CALENDAR_HREF.to_string(),
                                            t1,
                                        )]))
                                        .await;
                                }
                                if let Ok(t2) = client.get_tasks(&target_href).await {
                                    let _ = event_tx
                                        .send(AppEvent::TasksLoaded(vec![(target_href, t2)]))
                                        .await;
                                }
                            }
                            Err(e) => {
                                let _ = event_tx
                                    .send(AppEvent::Error(format!("Export failed: {}", e)))
                                    .await;
                            }
                        }
                    }
                }
            }
        }
    });

    // --- 5. UI LOOP ---
    loop {
        terminal.draw(|f| draw(f, &mut app_state))?;

        // A. Network Events
        if let Ok(event) = event_rx.try_recv() {
            match event {
                AppEvent::Status(s) => app_state.message = s,
                AppEvent::Error(s) => {
                    app_state.message = format!("Error: {}", s);
                    app_state.loading = false;
                }

                AppEvent::CalendarsLoaded(cals) => {
                    app_state.calendars = cals;

                    // Respect server's default first, otherwise fall back to Local
                    if let Some(def) = &default_cal {
                        if let Some(found) = app_state
                            .calendars
                            .iter()
                            .find(|c| c.name == *def || c.href == *def)
                        {
                            app_state.active_cal_href = Some(found.href.clone());
                        }
                    }

                    // If still nothing (or default was invalid), select Local
                    if app_state.active_cal_href.is_none() {
                        app_state.active_cal_href = Some(LOCAL_CALENDAR_HREF.to_string());
                    }

                    app_state.refresh_filtered_view();
                }

                AppEvent::TasksLoaded(results) => {
                    for (href, tasks) in results {
                        app_state.store.insert(href.clone(), tasks.clone());
                        let _ = Cache::save(&href, &tasks);
                    }
                    app_state.refresh_filtered_view();
                    // Don't set loading=false here blindly, maybe wait for "Ready" status or check logic
                    // But generally safe to say we have data now.
                    app_state.loading = false;
                }
            }
        }

        // B. User Input
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
                                let target_href = app_state.active_cal_href.clone().or_else(|| {
                                    app_state.calendars.first().map(|c| c.href.clone())
                                });

                                if let Some(href) = target_href {
                                    let mut task = Task::new(&summary, &app_state.tag_aliases);
                                    task.calendar_href = href.clone();

                                    // Optimistic Update
                                    if let Some(list) = app_state.store.calendars.get_mut(&href) {
                                        list.push(task.clone());
                                    }
                                    app_state.refresh_filtered_view();

                                    // Send full object
                                    let _ = action_tx.send(Action::CreateTask(task)).await;
                                }
                                app_state.mode = InputMode::Normal;
                                app_state.reset_input();
                            }
                        }
                        // ... (Esc, Char, Backspace handlers) ...
                        KeyCode::Esc => {
                            app_state.mode = InputMode::Normal;
                            app_state.reset_input();
                        }
                        KeyCode::Char(c) => app_state.enter_char(c),
                        KeyCode::Backspace => app_state.delete_char(),
                        _ => {}
                    },

                    InputMode::Editing => match key.code {
                        KeyCode::Enter => {
                            if let Some(idx) = app_state.editing_index
                                && let Some(view_task) = app_state.tasks.get(idx).cloned()
                            {
                                let cal_href = view_task.calendar_href.clone();
                                if let Some(list) = app_state.store.calendars.get_mut(&cal_href)
                                    && let Some(t) =
                                        list.iter_mut().find(|t| t.uid == view_task.uid)
                                {
                                    // Pass Aliases here
                                    t.apply_smart_input(
                                        &app_state.input_buffer,
                                        &app_state.tag_aliases,
                                    );
                                    let t_clone = t.clone();
                                    let _ = action_tx.send(Action::UpdateTask(t_clone)).await;
                                }
                                app_state.refresh_filtered_view();
                            }
                            app_state.mode = InputMode::Normal;
                            app_state.reset_input();
                            app_state.editing_index = None;
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

                    InputMode::EditingDescription => match key.code {
                        KeyCode::Enter => {
                            if let Some(idx) = app_state.editing_index
                                && let Some(view_task) = app_state.tasks.get(idx).cloned()
                            {
                                let cal_href = view_task.calendar_href.clone();
                                if let Some(list) = app_state.store.calendars.get_mut(&cal_href)
                                    && let Some(t) =
                                        list.iter_mut().find(|t| t.uid == view_task.uid)
                                {
                                    t.description = app_state.input_buffer.clone();
                                    let t_clone = t.clone();
                                    let _ = action_tx.send(Action::UpdateTask(t_clone)).await;
                                }
                                app_state.refresh_filtered_view();
                            }
                            app_state.mode = InputMode::Normal;
                            app_state.reset_input();
                            app_state.editing_index = None;
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
                            app_state.refresh_filtered_view();
                        }
                        KeyCode::Backspace => {
                            app_state.delete_char();
                            app_state.refresh_filtered_view();
                        }
                        _ => {}
                    },
                    InputMode::Moving => match key.code {
                        KeyCode::Esc => {
                            app_state.mode = InputMode::Normal;
                            app_state.message = String::new();
                        }
                        KeyCode::Down | KeyCode::Char('j') => app_state.next_move_target(),
                        KeyCode::Up | KeyCode::Char('k') => app_state.previous_move_target(),
                        KeyCode::Enter => {
                            if let Some(task) = app_state.get_selected_task().cloned() {
                                if let Some(idx) = app_state.move_selection_state.selected() {
                                    if let Some(target_cal) = app_state.move_targets.get(idx) {
                                        let target_href = target_cal.href.clone();

                                        // Optimistic Update
                                        if let Some(old_list) =
                                            app_state.store.calendars.get_mut(&task.calendar_href)
                                        {
                                            old_list.retain(|t| t.uid != task.uid);
                                        }
                                        let mut new_task_local = task.clone();
                                        new_task_local.calendar_href = target_href.clone();
                                        app_state
                                            .store
                                            .calendars
                                            .entry(target_href.clone())
                                            .or_default()
                                            .push(new_task_local);
                                        app_state.refresh_filtered_view();

                                        let _ = action_tx
                                            .send(Action::MoveTask(task, target_href))
                                            .await;
                                        app_state.message = format!(
                                            "Moving '{}'...",
                                            app_state
                                                .get_selected_task()
                                                .map_or("", |t| &t.summary)
                                        );
                                    }
                                }
                            }
                            app_state.mode = InputMode::Normal;
                        }
                        _ => {}
                    },

                    InputMode::Exporting => match key.code {
                        KeyCode::Esc => {
                            app_state.mode = InputMode::Normal;
                            app_state.message = String::new();
                        }
                        KeyCode::Down | KeyCode::Char('j') => app_state.next_export_target(),
                        KeyCode::Up | KeyCode::Char('k') => app_state.previous_export_target(),
                        KeyCode::Enter => {
                            if let Some(idx) = app_state.export_selection_state.selected() {
                                if let Some(target) = app_state.export_targets.get(idx) {
                                    // Send Action
                                    let _ = action_tx
                                        .send(Action::MigrateLocal(target.href.clone()))
                                        .await;
                                    // Optimistic: Clear local view immediately? Maybe safer to wait for confirm.
                                    app_state.mode = InputMode::Normal;
                                }
                            }
                        }
                        _ => {}
                    },

                    InputMode::Normal => match key.code {
                        KeyCode::Char('s') => {
                            if app_state.active_focus == Focus::Main
                                && let Some(task) = app_state.get_selected_task().cloned()
                            {
                                // Optimistic
                                let href = task.calendar_href.clone();
                                if let Some(list) = app_state.store.calendars.get_mut(&href)
                                    && let Some(t) = list.iter_mut().find(|t| t.uid == task.uid)
                                {
                                    if t.status == crate::model::TaskStatus::InProcess {
                                        t.status = crate::model::TaskStatus::NeedsAction;
                                    } else {
                                        t.status = crate::model::TaskStatus::InProcess;
                                    }
                                }
                                app_state.refresh_filtered_view();
                                let _ = action_tx.send(Action::MarkInProcess(task)).await;
                            }
                        }

                        KeyCode::Char('x') => {
                            if app_state.active_focus == Focus::Main
                                && let Some(task) = app_state.get_selected_task().cloned()
                            {
                                // Optimistic Update
                                let href = task.calendar_href.clone();
                                if let Some(list) = app_state.store.calendars.get_mut(&href)
                                    && let Some(t) = list.iter_mut().find(|t| t.uid == task.uid)
                                {
                                    if t.status == crate::model::TaskStatus::Cancelled {
                                        t.status = crate::model::TaskStatus::NeedsAction;
                                    } else {
                                        t.status = crate::model::TaskStatus::Cancelled;
                                    }
                                }
                                app_state.refresh_filtered_view();

                                // Send Network Action
                                let _ = action_tx.send(Action::MarkCancelled(task)).await;
                            }
                        }
                        KeyCode::Char('q') => {
                            let _ = action_tx.send(Action::Quit).await;
                            break;
                        }

                        KeyCode::Esc => {
                            app_state.reset_input();
                            app_state.refresh_filtered_view(); // <--- FIXED NAME
                            if app_state.yanked_uid.is_some() {
                                app_state.yanked_uid = None;
                                app_state.message = "Yank cleared.".to_string();
                            }
                        }

                        // 'c' to Make Child of Yanked
                        KeyCode::Char('c') => {
                            if let Some(parent_uid) = &app_state.yanked_uid {
                                if let Some(view_task) = app_state.get_selected_task().cloned() {
                                    // Check self-parenting
                                    if view_task.uid == *parent_uid {
                                        app_state.message = "Cannot be child of self!".to_string();
                                    } else {
                                        let href = view_task.calendar_href.clone();
                                        if let Some(list) = app_state.store.calendars.get_mut(&href)
                                            && let Some(t) =
                                                list.iter_mut().find(|t| t.uid == view_task.uid)
                                        {
                                            t.parent_uid = Some(parent_uid.clone());
                                            let t_clone = t.clone();
                                            let _ =
                                                action_tx.send(Action::UpdateTask(t_clone)).await;
                                        }
                                        app_state.refresh_filtered_view();
                                        app_state.message = "Parent set.".to_string();
                                    }
                                }
                            } else {
                                app_state.message = "Nothing yanked! Press 'y' first.".to_string();
                            }
                        }
                        KeyCode::Char('/') => {
                            app_state.mode = InputMode::Searching;
                            app_state.reset_input();
                        }
                        KeyCode::Tab => app_state.toggle_focus(),

                        KeyCode::Char('1') => {
                            app_state.sidebar_mode = SidebarMode::Calendars;
                            app_state.refresh_filtered_view();
                        }
                        KeyCode::Char('2') => {
                            app_state.sidebar_mode = SidebarMode::Categories;
                            app_state.refresh_filtered_view();
                        }
                        KeyCode::Char('m') => {
                            app_state.match_all_categories = !app_state.match_all_categories;
                            app_state.refresh_filtered_view();
                        }
                        KeyCode::Char('H') => {
                            app_state.hide_completed = !app_state.hide_completed;
                            app_state.refresh_filtered_view();
                        }
                        KeyCode::Char('M') => {
                            // Shift + m
                            if let Some(task) = app_state.get_selected_task() {
                                let current_href = task.calendar_href.clone();
                                app_state.move_targets = app_state
                                    .calendars
                                    .iter()
                                    .filter(|c| {
                                        c.href != current_href
                                            && !app_state.hidden_calendars.contains(&c.href)
                                    })
                                    .cloned()
                                    .collect();

                                if app_state.move_targets.is_empty() {
                                    app_state.message =
                                        "No other calendars to move to.".to_string();
                                } else {
                                    app_state.move_selection_state.select(Some(0));
                                    app_state.mode = InputMode::Moving;
                                    app_state.message =
                                        "Select a calendar and press Enter.".to_string();
                                }
                            }
                        }

                        KeyCode::Down | KeyCode::Char('j') => app_state.next(),
                        KeyCode::Up | KeyCode::Char('k') => app_state.previous(),
                        KeyCode::PageDown => app_state.jump_forward(10),
                        KeyCode::PageUp => app_state.jump_backward(10),

                        KeyCode::Enter => {
                            if app_state.active_focus == Focus::Sidebar {
                                match app_state.sidebar_mode {
                                    SidebarMode::Calendars => {
                                        if let Some(idx) = app_state.cal_state.selected() {
                                            let visible_cals: Vec<&CalendarListEntry> = app_state
                                                .calendars
                                                .iter()
                                                .filter(|c| {
                                                    !app_state.hidden_calendars.contains(&c.href)
                                                })
                                                .collect();

                                            if let Some(cal) = visible_cals.get(idx) {
                                                let href = cal.href.clone();
                                                app_state.active_cal_href = Some(href.clone());
                                                app_state.refresh_filtered_view(); // Updates view from Store immediately

                                                // ONLY send network request if NOT local
                                                if href != LOCAL_CALENDAR_HREF {
                                                    let _ = action_tx
                                                        .send(Action::SwitchCalendar(href))
                                                        .await;
                                                }
                                            }
                                        }
                                    }
                                    SidebarMode::Categories => {
                                        let cats = app_state.store.get_all_categories(
                                            app_state.hide_completed,
                                            app_state.hide_fully_completed_tags,
                                            &app_state.selected_categories,
                                            &app_state.hidden_calendars,
                                        );

                                        if let Some(idx) = app_state.cal_state.selected()
                                            && let Some(c) = cats.get(idx)
                                        {
                                            if app_state.selected_categories.contains(c) {
                                                app_state.selected_categories.remove(c);
                                            } else {
                                                app_state.selected_categories.insert(c.clone());
                                            }
                                            app_state.refresh_filtered_view();
                                        }
                                    }
                                }
                            }
                        }
                        KeyCode::Char('a') => {
                            app_state.mode = InputMode::Creating;
                            app_state.reset_input();
                            app_state.message =
                                "New Task (e.g. 'Buy Milk !1 @tomorrow')...".to_string();
                        }
                        KeyCode::Char('e') => {
                            if app_state.active_focus == Focus::Main {
                                let smart_str =
                                    app_state.get_selected_task().map(|t| t.to_smart_string());
                                if let Some(s) = smart_str {
                                    app_state.editing_index = app_state.list_state.selected();
                                    app_state.input_buffer = s;
                                    app_state.cursor_position =
                                        app_state.input_buffer.chars().count();
                                    app_state.mode = InputMode::Editing;
                                }
                            }
                        }
                        KeyCode::Char('E') => {
                            if app_state.active_focus == Focus::Main {
                                let desc =
                                    app_state.get_selected_task().map(|t| t.description.clone());
                                if let Some(d) = desc {
                                    app_state.editing_index = app_state.list_state.selected();
                                    app_state.input_buffer = d;
                                    app_state.cursor_position =
                                        app_state.input_buffer.chars().count();
                                    app_state.mode = InputMode::EditingDescription;
                                }
                            }
                        }
                        KeyCode::Char(' ') => {
                            if app_state.active_focus == Focus::Main
                                && let Some(task) = app_state.get_selected_task().cloned()
                            {
                                let cal_href = task.calendar_href.clone();
                                if let Some(list) = app_state.store.calendars.get_mut(&cal_href)
                                    && let Some(t) = list.iter_mut().find(|t| t.uid == task.uid)
                                {
                                    // Optimistic Toggle
                                    t.status = if t.status == crate::model::TaskStatus::Completed {
                                        crate::model::TaskStatus::NeedsAction
                                    } else {
                                        crate::model::TaskStatus::Completed
                                    };
                                    let t_flipped = t.clone();
                                    let _ = action_tx.send(Action::ToggleTask(t_flipped)).await;
                                }
                                app_state.refresh_filtered_view();
                            }
                        }
                        KeyCode::Char('d') => {
                            if app_state.active_focus == Focus::Main
                                && let Some(task) = app_state.get_selected_task().cloned()
                            {
                                let cal_href = task.calendar_href.clone();
                                if let Some(list) = app_state.store.calendars.get_mut(&cal_href) {
                                    list.retain(|t| t.uid != task.uid);
                                }
                                app_state.refresh_filtered_view();
                                let _ = action_tx.send(Action::DeleteTask(task)).await;
                            }
                        }

                        KeyCode::Char('+') => {
                            if app_state.active_focus == Focus::Main
                                && let Some(view_task) = app_state.get_selected_task().cloned()
                            {
                                let cal_href = view_task.calendar_href.clone();
                                if let Some(list) = app_state.store.calendars.get_mut(&cal_href)
                                    && let Some(t) =
                                        list.iter_mut().find(|t| t.uid == view_task.uid)
                                {
                                    let new_prio = match t.priority {
                                        0 => 9,
                                        9 => 5,
                                        5 => 1,
                                        1 => 1,
                                        _ => 5,
                                    };
                                    if new_prio != t.priority {
                                        t.priority = new_prio;
                                        let t_clone = t.clone();
                                        let _ = action_tx.send(Action::UpdateTask(t_clone)).await;
                                    }
                                }
                                app_state.refresh_filtered_view();
                            }
                        }
                        KeyCode::Char('-') => {
                            if app_state.active_focus == Focus::Main
                                && let Some(view_task) = app_state.get_selected_task().cloned()
                            {
                                let cal_href = view_task.calendar_href.clone();
                                if let Some(list) = app_state.store.calendars.get_mut(&cal_href)
                                    && let Some(t) =
                                        list.iter_mut().find(|t| t.uid == view_task.uid)
                                {
                                    let new_prio = match t.priority {
                                        1 => 5,
                                        5 => 9,
                                        9 => 0,
                                        0 => 0,
                                        _ => 0,
                                    };
                                    if new_prio != t.priority {
                                        t.priority = new_prio;
                                        let t_clone = t.clone();
                                        let _ = action_tx.send(Action::UpdateTask(t_clone)).await;
                                    }
                                }
                                app_state.refresh_filtered_view();
                            }
                        }
                        KeyCode::Char('.') | KeyCode::Char('>') => {
                            if app_state.active_focus == Focus::Main
                                && let Some(idx) = app_state.list_state.selected()
                                && idx > 0
                                && idx < app_state.tasks.len()
                            {
                                let parent_uid = app_state.tasks[idx - 1].uid.clone();
                                let current_uid = app_state.tasks[idx].uid.clone();
                                let cal_href = app_state.tasks[idx].calendar_href.clone();
                                if let Some(list) = app_state.store.calendars.get_mut(&cal_href)
                                    && let Some(t) = list.iter_mut().find(|t| t.uid == current_uid)
                                    && t.parent_uid != Some(parent_uid.clone())
                                {
                                    t.parent_uid = Some(parent_uid);
                                    let t_clone = t.clone();
                                    let _ = action_tx.send(Action::UpdateTask(t_clone)).await;
                                }
                                app_state.refresh_filtered_view();
                            }
                        }
                        KeyCode::Char(',') | KeyCode::Char('<') => {
                            if app_state.active_focus == Focus::Main
                                && let Some(view_task) = app_state.get_selected_task().cloned()
                            {
                                let cal_href = view_task.calendar_href.clone();
                                if let Some(list) = app_state.store.calendars.get_mut(&cal_href)
                                    && let Some(t) =
                                        list.iter_mut().find(|t| t.uid == view_task.uid)
                                    && t.parent_uid.is_some()
                                {
                                    t.parent_uid = None;
                                    let t_clone = t.clone();
                                    let _ = action_tx.send(Action::UpdateTask(t_clone)).await;
                                }
                                app_state.refresh_filtered_view();
                            }
                        }
                        KeyCode::Char('y') => {
                            let selection = app_state
                                .get_selected_task()
                                .map(|t| (t.uid.clone(), t.summary.clone()));

                            if let Some((uid, summary)) = selection {
                                app_state.yanked_uid = Some(uid);
                                app_state.message = format!("Yanked: {}", summary);
                            }
                        }
                        KeyCode::Char('r') => {
                            let _ = action_tx.send(Action::Refresh).await;
                        }
                        KeyCode::Char('b') => {
                            // "Block this task with the yanked one"
                            if let Some(yanked) = &app_state.yanked_uid {
                                if let Some(current) = app_state.get_selected_task() {
                                    if current.uid == *yanked {
                                        app_state.message = "Cannot depend on self!".to_string();
                                    } else {
                                        // Clone to modify
                                        let mut t_clone = current.clone();
                                        // Add dependency if not exists
                                        if !t_clone.dependencies.contains(yanked) {
                                            t_clone.dependencies.push(yanked.clone());

                                            // Optimistic update
                                            let href = t_clone.calendar_href.clone();
                                            if let Some(list) =
                                                app_state.store.calendars.get_mut(&href)
                                                && let Some(t) =
                                                    list.iter_mut().find(|t| t.uid == t_clone.uid)
                                            {
                                                t.dependencies.push(yanked.clone());
                                            }

                                            let _ =
                                                action_tx.send(Action::UpdateTask(t_clone)).await;
                                            app_state.refresh_filtered_view();
                                            app_state.message = "Dependency added.".to_string();
                                        }
                                    }
                                }
                            } else {
                                app_state.message =
                                    "Nothing yanked! Press 'y' on a task first.".to_string();
                            }
                        }
                        KeyCode::Char('X') => {
                            // Shift + x
                            // Only allow if active calendar is Local
                            if app_state.active_cal_href.as_deref() == Some(LOCAL_CALENDAR_HREF) {
                                // Filter targets (All remote cals)
                                app_state.export_targets = app_state
                                    .calendars
                                    .iter()
                                    .filter(|c| {
                                        c.href != LOCAL_CALENDAR_HREF
                                            && !app_state.hidden_calendars.contains(&c.href)
                                    })
                                    .cloned()
                                    .collect();

                                if app_state.export_targets.is_empty() {
                                    app_state.message =
                                        "No remote calendars to export to.".to_string();
                                } else {
                                    app_state.export_selection_state.select(Some(0));
                                    app_state.mode = InputMode::Exporting;
                                    app_state.message = "Select export target...".to_string();
                                }
                            } else {
                                app_state.message =
                                    "Export only available from Local calendar.".to_string();
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
