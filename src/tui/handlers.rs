// File: src/tui/handlers.rs
use crate::config::Config;
use crate::model::{Task, TaskStatus, extract_inline_aliases};
use crate::storage::LOCAL_CALENDAR_HREF;
use crate::tui::action::{Action, AppEvent, SidebarMode};
use crate::tui::state::{AppState, Focus, InputMode};
use crossterm::event::{KeyCode, KeyEvent};
use tokio::sync::mpsc::Sender;

pub fn handle_app_event(state: &mut AppState, event: AppEvent, default_cal: &Option<String>) {
    match event {
        AppEvent::Status(s) => state.message = s,
        AppEvent::Error(s) => {
            state.message = format!("Error: {}", s);
            state.loading = false;
        }
        AppEvent::CalendarsLoaded(cals) => {
            state.calendars = cals;

            // Unhide default calendar on load
            if let Some(def) = default_cal
                && let Some(found) = state
                    .calendars
                    .iter()
                    .find(|c| c.name == *def || c.href == *def)
            {
                if state.hidden_calendars.contains(&found.href) {
                    state.hidden_calendars.remove(&found.href);
                }
                state.active_cal_href = Some(found.href.clone());
            }

            if state.active_cal_href.is_none() {
                state.active_cal_href = Some(LOCAL_CALENDAR_HREF.to_string());
            }
            state.refresh_filtered_view();
        }
        AppEvent::TasksLoaded(results) => {
            for (href, tasks) in results {
                state.store.insert(href, tasks);
            }
            state.refresh_filtered_view();
            state.loading = false;
        }
    }
}

pub async fn handle_key_event(
    key: KeyEvent,
    state: &mut AppState,
    action_tx: &Sender<Action>,
) -> Option<Action> {
    match state.mode {
        InputMode::Creating => match key.code {
            KeyCode::Enter if !state.input_buffer.is_empty() => {
                // --- 1. Extract Inline Aliases ---
                let (clean_input, new_aliases) = extract_inline_aliases(&state.input_buffer);

                if !new_aliases.is_empty() {
                    for (key, tags) in new_aliases {
                        state.tag_aliases.insert(key.clone(), tags.clone());

                        // Shared Logic: Retroactive Update
                        let modified = state.store.apply_alias_retroactively(&key, &tags);

                        // Dispatch Updates
                        for t in modified {
                            let _ = action_tx.send(Action::UpdateTask(t)).await;
                        }
                    }
                    // Persist Aliases
                    if let Ok(mut cfg) = Config::load() {
                        cfg.tag_aliases = state.tag_aliases.clone();
                        let _ = cfg.save();
                    }
                }

                // --- 2. Existing Logic with Clean Input ---

                if clean_input.starts_with('#')
                    && !clean_input.trim().contains(' ')
                    && state.creating_child_of.is_none()
                {
                    let was_alias_def = state.input_buffer.contains('=');

                    if !was_alias_def {
                        let tag = clean_input.trim().trim_start_matches('#').to_string();
                        if !tag.is_empty() {
                            state.sidebar_mode = SidebarMode::Categories;
                            state.selected_categories.clear();
                            state.selected_categories.insert(tag);
                            state.mode = InputMode::Normal;
                            state.reset_input();
                            state.refresh_filtered_view();
                            return None;
                        }
                    } else {
                        state.mode = InputMode::Normal;
                        state.reset_input();
                        state.message = "Alias updated.".to_string();
                        return None;
                    }
                }

                let target_href = state
                    .active_cal_href
                    .clone()
                    .or_else(|| state.calendars.first().map(|c| c.href.clone()));

                if let Some(href) = target_href {
                    let mut task = Task::new(&clean_input, &state.tag_aliases);
                    task.calendar_href = href.clone();
                    task.parent_uid = state.creating_child_of.clone();

                    state.store.add_task(task.clone());
                    state.refresh_filtered_view();

                    state.mode = InputMode::Normal;
                    state.reset_input();
                    state.creating_child_of = None;
                    return Some(Action::CreateTask(task));
                }
                state.mode = InputMode::Normal;
                state.reset_input();
            }
            KeyCode::Esc => {
                state.mode = InputMode::Normal;
                state.reset_input();
            }
            KeyCode::Char(c) => state.enter_char(c),
            KeyCode::Backspace => state.delete_char(),
            KeyCode::Left => state.move_cursor_left(),
            KeyCode::Right => state.move_cursor_right(),
            _ => {}
        },
        InputMode::Editing => match key.code {
            KeyCode::Enter => {
                let (clean_input, new_aliases) = extract_inline_aliases(&state.input_buffer);
                if !new_aliases.is_empty() {
                    for (k, v) in new_aliases {
                        state.tag_aliases.insert(k.clone(), v.clone());
                        let modified = state.store.apply_alias_retroactively(&k, &v);
                        for mod_t in modified {
                            let _ = action_tx.send(Action::UpdateTask(mod_t)).await;
                        }
                    }
                    if let Ok(mut cfg) = Config::load() {
                        cfg.tag_aliases = state.tag_aliases.clone();
                        let _ = cfg.save();
                    }
                }

                let target_uid = state
                    .editing_index
                    .and_then(|idx| state.tasks.get(idx).map(|t| t.uid.clone()));

                if let Some(uid) = target_uid
                    && let Some((t, _)) = state.store.get_task_mut(&uid)
                {
                    t.apply_smart_input(&clean_input, &state.tag_aliases);
                    let clone = t.clone();
                    state.refresh_filtered_view();
                    state.mode = InputMode::Normal;
                    state.reset_input();
                    return Some(Action::UpdateTask(clone));
                }
                state.mode = InputMode::Normal;
            }
            KeyCode::Esc => {
                state.mode = InputMode::Normal;
                state.reset_input();
            }
            KeyCode::Char(c) => state.enter_char(c),
            KeyCode::Backspace => state.delete_char(),
            KeyCode::Left => state.move_cursor_left(),
            KeyCode::Right => state.move_cursor_right(),
            _ => {}
        },
        InputMode::EditingDescription => match key.code {
            KeyCode::Enter => {
                if key.modifiers.contains(crossterm::event::KeyModifiers::ALT)
                    || key
                        .modifiers
                        .contains(crossterm::event::KeyModifiers::SHIFT)
                {
                    state.enter_char('\n');
                } else {
                    let target_uid = state
                        .editing_index
                        .and_then(|idx| state.tasks.get(idx).map(|t| t.uid.clone()));

                    if let Some(uid) = target_uid
                        && let Some((t, _)) = state.store.get_task_mut(&uid)
                    {
                        t.description = state.input_buffer.clone();
                        let clone = t.clone();
                        state.refresh_filtered_view();
                        state.mode = InputMode::Normal;
                        state.reset_input();
                        return Some(Action::UpdateTask(clone));
                    }
                    state.mode = InputMode::Normal;
                    state.reset_input();
                }
            }
            KeyCode::Esc => {
                state.mode = InputMode::Normal;
                state.reset_input();
            }
            KeyCode::Char(c) => state.enter_char(c),
            KeyCode::Backspace => state.delete_char(),
            KeyCode::Left => state.move_cursor_left(),
            KeyCode::Right => state.move_cursor_right(),
            _ => {}
        },
        InputMode::Normal => match key.code {
            KeyCode::Char('?') => state.show_full_help = !state.show_full_help,
            KeyCode::Char('q') => return Some(Action::Quit),
            KeyCode::Char('r') => return Some(Action::Refresh),

            KeyCode::Char(' ') => {
                if state.active_focus == Focus::Main {
                    if let Some(uid) = state.get_selected_task().map(|t| t.uid.clone())
                        && let Some(updated) = state.store.toggle_task(&uid)
                    {
                        state.refresh_filtered_view();
                        return Some(Action::ToggleTask(updated));
                    }
                } else if state.active_focus == Focus::Sidebar
                    && state.sidebar_mode == SidebarMode::Calendars
                {
                    let target_cal = if let Some(idx) = state.cal_state.selected() {
                        let filtered = state.get_filtered_calendars();
                        filtered.get(idx).map(|c| c.href.clone())
                    } else {
                        None
                    };

                    if let Some(href) = target_cal
                        && state.active_cal_href.as_ref() != Some(&href)
                    {
                        if state.hidden_calendars.contains(&href) {
                            state.hidden_calendars.remove(&href);
                            let _ = action_tx.send(Action::ToggleCalendarVisibility(href)).await;
                        } else {
                            state.hidden_calendars.insert(href);
                        }
                        state.refresh_filtered_view();
                    }
                }
            }
            KeyCode::Char('s') => {
                if let Some(uid) = state.get_selected_task().map(|t| t.uid.clone())
                    && let Some(updated) = state.store.set_status(&uid, TaskStatus::InProcess)
                {
                    state.refresh_filtered_view();
                    return Some(Action::MarkInProcess(updated));
                }
            }
            KeyCode::Char('x') => {
                if let Some(uid) = state.get_selected_task().map(|t| t.uid.clone())
                    && let Some(updated) = state.store.set_status(&uid, TaskStatus::Cancelled)
                {
                    state.refresh_filtered_view();
                    return Some(Action::MarkCancelled(updated));
                }
            }
            KeyCode::Char('+') => {
                if let Some(uid) = state.get_selected_task().map(|t| t.uid.clone())
                    && let Some(updated) = state.store.change_priority(&uid, 1)
                {
                    state.refresh_filtered_view();
                    return Some(Action::UpdateTask(updated));
                }
            }
            KeyCode::Char('-') => {
                if let Some(uid) = state.get_selected_task().map(|t| t.uid.clone())
                    && let Some(updated) = state.store.change_priority(&uid, -1)
                {
                    state.refresh_filtered_view();
                    return Some(Action::UpdateTask(updated));
                }
            }
            KeyCode::Char('d') => {
                if let Some(uid) = state.get_selected_task().map(|t| t.uid.clone())
                    && let Some(deleted) = state.store.delete_task(&uid)
                {
                    state.refresh_filtered_view();
                    return Some(Action::DeleteTask(deleted));
                }
            }
            KeyCode::Char('c') => {
                let data = if let Some(parent_uid) = &state.yanked_uid
                    && let Some(view_task) = state.get_selected_task()
                {
                    Some((view_task.uid.clone(), parent_uid.clone()))
                } else {
                    None
                };

                if let Some((child_uid, parent_uid)) = data {
                    if child_uid == parent_uid {
                        state.message = "Cannot be child of self!".to_string();
                    } else if let Some(updated) =
                        state.store.set_parent(&child_uid, Some(parent_uid))
                    {
                        state.yanked_uid = None; // Auto-unlink after action
                        state.refresh_filtered_view();
                        return Some(Action::UpdateTask(updated));
                    }
                }
            }
            KeyCode::Char('C') => {
                if state.active_focus == Focus::Main
                    && let Some(task) = state.get_selected_task()
                {
                    let uid = task.uid.clone();
                    let summary = task.summary.clone();

                    // --- Auto-fill tags from parent ---
                    let mut initial_input = String::new();
                    for cat in &task.categories {
                        initial_input.push_str(&format!("#{} ", cat));
                    }

                    state.input_buffer = initial_input;
                    state.cursor_position = state.input_buffer.len();

                    state.mode = InputMode::Creating;
                    state.creating_child_of = Some(uid);
                    state.message = format!("New Child of '{}'...", summary);
                }
            }
            KeyCode::Char('y') => {
                if let Some(t) = state.get_selected_task() {
                    let uid = t.uid.clone();
                    let summary = t.summary.clone();
                    state.yanked_uid = Some(uid);
                    state.message = format!("Yanked: {}", summary);
                }
            }
            KeyCode::Char('b') => {
                let data = if let Some(yanked) = &state.yanked_uid
                    && let Some(current) = state.get_selected_task()
                {
                    Some((current.uid.clone(), yanked.clone()))
                } else {
                    None
                };

                if let Some((curr_uid, yanked_uid)) = data {
                    if curr_uid == yanked_uid {
                        state.message = "Cannot depend on self!".to_string();
                    } else if let Some(updated) = state.store.add_dependency(&curr_uid, yanked_uid)
                    {
                        state.yanked_uid = None; // Auto-unlink after action
                        state.refresh_filtered_view();
                        return Some(Action::UpdateTask(updated));
                    }
                }
            }
            KeyCode::Char('.') | KeyCode::Char('>') => {
                if state.active_focus == Focus::Main
                    && let Some(idx) = state.list_state.selected()
                    && idx > 0
                    && idx < state.tasks.len()
                {
                    let parent_uid = state.tasks[idx - 1].uid.clone();
                    let current_uid = state.tasks[idx].uid.clone();
                    if let Some(updated) = state.store.set_parent(&current_uid, Some(parent_uid)) {
                        state.refresh_filtered_view();
                        return Some(Action::UpdateTask(updated));
                    }
                }
            }
            KeyCode::Char(',') | KeyCode::Char('<') => {
                if state.active_focus == Focus::Main
                    && let Some(view_task) = state.get_selected_task()
                    && view_task.parent_uid.is_some()
                {
                    let uid = view_task.uid.clone();
                    if let Some(updated) = state.store.set_parent(&uid, None) {
                        state.refresh_filtered_view();
                        return Some(Action::UpdateTask(updated));
                    }
                }
            }
            KeyCode::Char('X') => {
                if state.active_cal_href.as_deref() == Some(LOCAL_CALENDAR_HREF) {
                    state.export_targets = state
                        .calendars
                        .iter()
                        .filter(|c| {
                            c.href != LOCAL_CALENDAR_HREF
                                && !state.disabled_calendars.contains(&c.href)
                        })
                        .cloned()
                        .collect();
                    if !state.export_targets.is_empty() {
                        state.export_selection_state.select(Some(0));
                        state.mode = InputMode::Exporting;
                    }
                }
            }
            KeyCode::Char('M') => {
                if let Some(task) = state.get_selected_task() {
                    let current_href = task.calendar_href.clone();
                    state.move_targets = state
                        .calendars
                        .iter()
                        .filter(|c| {
                            c.href != current_href && !state.disabled_calendars.contains(&c.href)
                        })
                        .cloned()
                        .collect();
                    if !state.move_targets.is_empty() {
                        state.move_selection_state.select(Some(0));
                        state.mode = InputMode::Moving;
                        state.message = "Select a calendar and press Enter.".to_string();
                    }
                }
            }
            KeyCode::Down | KeyCode::Char('j') => state.next(),
            KeyCode::Up | KeyCode::Char('k') => state.previous(),
            KeyCode::PageDown => state.jump_forward(10),
            KeyCode::PageUp => state.jump_backward(10),
            KeyCode::Tab => state.toggle_focus(),
            KeyCode::Char('1') => {
                state.sidebar_mode = SidebarMode::Calendars;
                state.refresh_filtered_view();
            }
            KeyCode::Char('2') => {
                state.sidebar_mode = SidebarMode::Categories;
                state.refresh_filtered_view();
            }
            KeyCode::Char('m') => {
                state.match_all_categories = !state.match_all_categories;
                state.refresh_filtered_view();
            }
            KeyCode::Char('H') => {
                state.hide_completed = !state.hide_completed;
                state.refresh_filtered_view();
            }
            KeyCode::Char('*') => {
                if state.active_focus == Focus::Sidebar {
                    match state.sidebar_mode {
                        SidebarMode::Calendars => {
                            let enabled_count = state
                                .calendars
                                .iter()
                                .filter(|c| !state.disabled_calendars.contains(&c.href))
                                .count();
                            let visible_count = state
                                .calendars
                                .iter()
                                .filter(|c| {
                                    !state.disabled_calendars.contains(&c.href)
                                        && !state.hidden_calendars.contains(&c.href)
                                })
                                .count();
                            if visible_count == enabled_count {
                                for cal in &state.calendars {
                                    if state.active_cal_href.as_ref() != Some(&cal.href) {
                                        state.hidden_calendars.insert(cal.href.clone());
                                    }
                                }
                            } else {
                                state.hidden_calendars.clear();
                                let _ = action_tx.send(Action::Refresh).await;
                            }
                        }
                        SidebarMode::Categories => {
                            state.selected_categories.clear();
                        }
                    }
                    state.refresh_filtered_view();
                }
            }
            KeyCode::Right => {
                if state.active_focus == Focus::Sidebar
                    && state.sidebar_mode == SidebarMode::Calendars
                {
                    let target_href = if let Some(idx) = state.cal_state.selected() {
                        let filtered = state.get_filtered_calendars();
                        filtered.get(idx).map(|c| c.href.clone())
                    } else {
                        None
                    };

                    if let Some(href) = target_href {
                        state.active_cal_href = Some(href.clone());
                        state.hidden_calendars.clear();
                        for c in &state.calendars {
                            if c.href != href {
                                state.hidden_calendars.insert(c.href.clone());
                            }
                        }
                        state.refresh_filtered_view();
                        if href != LOCAL_CALENDAR_HREF {
                            return Some(Action::IsolateCalendar(href));
                        }
                    }
                } else if state.mode == InputMode::Editing {
                    state.move_cursor_right();
                }
            }
            KeyCode::Enter => {
                if state.active_focus == Focus::Sidebar {
                    match state.sidebar_mode {
                        SidebarMode::Calendars => {
                            let target_href = if let Some(idx) = state.cal_state.selected() {
                                let filtered = state.get_filtered_calendars();
                                filtered.get(idx).map(|c| c.href.clone())
                            } else {
                                None
                            };

                            if let Some(href) = target_href {
                                state.active_cal_href = Some(href.clone());
                                state.hidden_calendars.remove(&href);
                                state.refresh_filtered_view();
                                if href != LOCAL_CALENDAR_HREF {
                                    return Some(Action::SwitchCalendar(href));
                                }
                            }
                        }
                        SidebarMode::Categories => {
                            let cats = state.store.get_all_categories(
                                state.hide_completed,
                                state.hide_fully_completed_tags,
                                &state.selected_categories,
                                &state.hidden_calendars,
                            );
                            if let Some(idx) = state.cal_state.selected()
                                && let Some((c, _)) = cats.get(idx)
                            {
                                let c_clone = c.clone();
                                if state.selected_categories.contains(&c_clone) {
                                    state.selected_categories.remove(&c_clone);
                                } else {
                                    state.selected_categories.insert(c_clone);
                                }
                                state.refresh_filtered_view();
                            }
                        }
                    }
                }
            }
            KeyCode::Char('/') => {
                state.mode = InputMode::Searching;
                state.reset_input();
            }
            KeyCode::Char('a') => {
                state.mode = InputMode::Creating;
                state.reset_input();
                state.message = "New Task...".to_string();
            }
            KeyCode::Char('e') => {
                if let Some(t) = state.get_selected_task() {
                    state.input_buffer = t.to_smart_string();
                    state.cursor_position = state.input_buffer.len();
                    state.editing_index = state.list_state.selected();
                    state.mode = InputMode::Editing;
                }
            }
            KeyCode::Char('E') => {
                if state.active_focus == Focus::Main
                    && let Some(t) = state.get_selected_task()
                {
                    state.input_buffer = t.description.clone();
                    state.cursor_position = state.input_buffer.len();
                    state.editing_index = state.list_state.selected();
                    state.mode = InputMode::EditingDescription;
                }
            }
            _ => {}
        },
        InputMode::Moving => match key.code {
            KeyCode::Esc => {
                state.mode = InputMode::Normal;
                state.message = String::new();
            }
            KeyCode::Down | KeyCode::Char('j') => state.next_move_target(),
            KeyCode::Up | KeyCode::Char('k') => state.previous_move_target(),
            KeyCode::Enter => {
                let data = if let Some(task) = state.get_selected_task()
                    && let Some(idx) = state.move_selection_state.selected()
                    && let Some(target_cal) = state.move_targets.get(idx)
                {
                    Some((task.uid.clone(), target_cal.href.clone()))
                } else {
                    None
                };

                if let Some((uid, target_href)) = data
                    && let Some(updated) = state.store.move_task(&uid, target_href.clone())
                {
                    state.refresh_filtered_view();
                    state.message = "Moving task...".to_string();
                    state.mode = InputMode::Normal;
                    return Some(Action::MoveTask(updated, target_href));
                }
                state.mode = InputMode::Normal;
            }
            _ => {}
        },
        InputMode::Exporting => match key.code {
            KeyCode::Esc => {
                state.mode = InputMode::Normal;
                state.message = String::new();
            }
            KeyCode::Down | KeyCode::Char('j') => state.next_export_target(),
            KeyCode::Up | KeyCode::Char('k') => state.previous_export_target(),
            KeyCode::Enter => {
                if let Some(idx) = state.export_selection_state.selected()
                    && let Some(target) = state.export_targets.get(idx)
                {
                    let href = target.href.clone();
                    state.mode = InputMode::Normal;
                    return Some(Action::MigrateLocal(href));
                }
            }
            _ => {}
        },
        _ => {}
    }
    None
}
