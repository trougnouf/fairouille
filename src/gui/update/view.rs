// File: ./src/gui/update/view.rs
use crate::gui::async_ops::*;
use crate::gui::message::Message;
use crate::gui::state::{AppState, GuiApp, ResizeDirection, SidebarMode};
use crate::gui::update::common::{refresh_filtered_tasks, save_config};
use iced::{Point, Size, Task, window};

pub fn handle(app: &mut GuiApp, message: Message) -> Task<Message> {
    match message {
        Message::TabPressed(shift_held) => {
            if shift_held {
                iced::widget::focus_previous()
            } else {
                iced::widget::focus_next()
            }
        }
        Message::DismissError => {
            app.error_msg = None;
            Task::none()
        }
        Message::ToggleAllCalendars(show_all) => {
            if show_all {
                app.hidden_calendars.clear();
            } else {
                for cal in &app.calendars {
                    if app.active_cal_href.as_ref() != Some(&cal.href) {
                        app.hidden_calendars.insert(cal.href.clone());
                    }
                }
            }
            save_config(app);
            refresh_filtered_tasks(app);
            Task::perform(async { Ok::<(), String>(()) }, |_| Message::Refresh)
        }
        Message::IsolateCalendar(href) => {
            if app.sidebar_mode == SidebarMode::Categories {
                app.sidebar_mode = SidebarMode::Calendars;
            }
            app.active_cal_href = Some(href.clone());
            app.hidden_calendars.clear();
            for cal in &app.calendars {
                if cal.href != href {
                    app.hidden_calendars.insert(cal.href.clone());
                }
            }
            if app.disabled_calendars.contains(&href) {
                app.disabled_calendars.remove(&href);
            }
            save_config(app);
            refresh_filtered_tasks(app);

            if let Some(client) = &app.client {
                if !app.store.calendars.contains_key(&href) {
                    app.loading = true;
                }
                return Task::perform(
                    async_fetch_wrapper(client.clone(), href),
                    Message::TasksRefreshed,
                );
            }
            Task::none()
        }
        Message::SidebarModeChanged(mode) => {
            app.sidebar_mode = mode;
            refresh_filtered_tasks(app);
            Task::none()
        }
        Message::CategoryToggled(cat) => {
            if app.selected_categories.contains(&cat) {
                app.selected_categories.remove(&cat);
            } else {
                app.selected_categories.insert(cat);
            }
            refresh_filtered_tasks(app);
            Task::none()
        }
        Message::ClearAllTags => {
            app.selected_categories.clear();
            refresh_filtered_tasks(app);
            Task::none()
        }
        Message::CategoryMatchModeChanged(val) => {
            app.match_all_categories = val;
            refresh_filtered_tasks(app);
            Task::none()
        }
        Message::ToggleHideCompleted(val) => {
            app.hide_completed = val;
            save_config(app);
            refresh_filtered_tasks(app);
            Task::none()
        }
        Message::ToggleHideFullyCompletedTags(val) => {
            app.hide_fully_completed_tags = val;
            save_config(app);
            refresh_filtered_tasks(app);
            Task::none()
        }
        Message::SelectCalendar(href) => {
            if app.sidebar_mode == SidebarMode::Categories {
                app.sidebar_mode = SidebarMode::Calendars;
            }
            app.active_cal_href = Some(href.clone());
            if app.hidden_calendars.contains(&href) {
                app.hidden_calendars.remove(&href);
                save_config(app);
            }
            refresh_filtered_tasks(app);
            if let Some(client) = &app.client {
                if !app.store.calendars.contains_key(&href) {
                    app.loading = true;
                }
                return Task::perform(
                    async_fetch_wrapper(client.clone(), href),
                    Message::TasksRefreshed,
                );
            }
            Task::none()
        }
        Message::ToggleCalendarDisabled(href, is_disabled) => {
            if is_disabled {
                app.disabled_calendars.insert(href.clone());
                if app.active_cal_href.as_ref() == Some(&href) {
                    app.active_cal_href = None;
                }
            } else {
                app.disabled_calendars.remove(&href);
            }
            save_config(app);
            refresh_filtered_tasks(app);
            Task::none()
        }
        Message::ToggleCalendarVisibility(href, is_visible) => {
            if !is_visible && app.active_cal_href.as_ref() == Some(&href) {
                return Task::none();
            }
            if is_visible {
                app.hidden_calendars.remove(&href);
            } else {
                app.hidden_calendars.insert(href);
            }
            save_config(app);
            refresh_filtered_tasks(app);
            Task::none()
        }
        Message::SearchChanged(val) => {
            app.search_value = val;
            refresh_filtered_tasks(app);
            Task::none()
        }
        Message::SetMinDuration(val) => {
            app.filter_min_duration = val;
            refresh_filtered_tasks(app);
            Task::none()
        }
        Message::SetMaxDuration(val) => {
            app.filter_max_duration = val;
            refresh_filtered_tasks(app);
            Task::none()
        }
        Message::ToggleIncludeUnsetDuration(val) => {
            app.filter_include_unset_duration = val;
            refresh_filtered_tasks(app);
            Task::none()
        }
        Message::ToggleDetails(uid) => {
            if app.expanded_tasks.contains(&uid) {
                app.expanded_tasks.remove(&uid);
            } else {
                app.expanded_tasks.insert(uid);
            }
            Task::none()
        }
        Message::OpenHelp => {
            app.state = AppState::Help;
            Task::none()
        }
        Message::CloseHelp => {
            app.state = AppState::Active;
            Task::none()
        }
        Message::WindowDragged => window::get_latest().then(|id| {
            if let Some(id) = id {
                window::drag(id)
            } else {
                Task::none()
            }
        }),
        Message::MinimizeWindow => window::get_latest().then(|id| {
            if let Some(id) = id {
                window::minimize(id, true)
            } else {
                Task::none()
            }
        }),
        Message::CloseWindow => window::get_latest().then(|id| {
            if let Some(id) = id {
                window::close(id)
            } else {
                Task::none()
            }
        }),
        // Resize Logic
        Message::ResizeStart(direction) => {
            app.resize_direction = Some(direction);
            // We do not set last_cursor_pos here anymore, we rely on the implicit delta from the edge
            // or we could snap it. For manual resize implementation, capturing the start is enough.
            Task::none()
        }
        Message::ResizeUpdate(cursor_pos) => {
            if let Some(dir) = app.resize_direction {
                let current_size = app.current_window_size;
                let current_win_pos = app.current_window_pos;

                let min_width = 400.0;
                let min_height = 300.0;

                let mut new_width = current_size.width;
                let mut new_height = current_size.height;
                let mut new_x = current_win_pos.x;
                let mut new_y = current_win_pos.y;
                let mut move_needed = false;

                // Delta X
                // For Right/East: cursor_pos.x is the new width
                // For Left/West: cursor_pos.x is how much we shift X and shrink Width

                match dir {
                    ResizeDirection::East
                    | ResizeDirection::NorthEast
                    | ResizeDirection::SouthEast => {
                        new_width = cursor_pos.x.max(min_width);
                    }
                    ResizeDirection::West
                    | ResizeDirection::NorthWest
                    | ResizeDirection::SouthWest => {
                        // cursor_pos.x is relative to current window left edge (0).
                        // If -5, we move window -5 and increase width +5.
                        // However, we must ensure we don't shrink below min_width.
                        let potential_width = current_size.width - cursor_pos.x;
                        if potential_width >= min_width {
                            new_width = potential_width;
                            new_x += cursor_pos.x;
                            move_needed = true;
                        }
                    }
                    _ => {}
                }

                // Delta Y
                match dir {
                    ResizeDirection::South
                    | ResizeDirection::SouthEast
                    | ResizeDirection::SouthWest => {
                        new_height = cursor_pos.y.max(min_height);
                    }
                    ResizeDirection::North
                    | ResizeDirection::NorthEast
                    | ResizeDirection::NorthWest => {
                        let potential_height = current_size.height - cursor_pos.y;
                        if potential_height >= min_height {
                            new_height = potential_height;
                            new_y += cursor_pos.y;
                            move_needed = true;
                        }
                    }
                    _ => {}
                }

                // Apply changes
                let mut tasks = Vec::new();

                // Only resize if dimensions changed
                if new_width != current_size.width || new_height != current_size.height {
                    tasks.push(window::get_latest().then(move |id| {
                        if let Some(id) = id {
                            window::resize(id, Size::new(new_width, new_height))
                        } else {
                            Task::none()
                        }
                    }));
                }

                // Only move if needed
                if move_needed {
                    tasks.push(window::get_latest().then(move |id| {
                        if let Some(id) = id {
                            window::move_to(id, Point::new(new_x, new_y))
                        } else {
                            Task::none()
                        }
                    }));
                }

                return Task::batch(tasks);
            }
            Task::none()
        }
        Message::ResizeEnd => {
            app.resize_direction = None;
            Task::none()
        }
        Message::WindowResized(size) => {
            app.current_window_size = size;
            Task::none()
        }
        Message::WindowMoved(point) => {
            app.current_window_pos = point;
            Task::none()
        }
        _ => Task::none(),
    }
}
