// File: src/gui/update/tasks.rs
use crate::gui::async_ops::*;
use crate::gui::message::Message;
use crate::gui::state::{GuiApp, SidebarMode};
use crate::gui::update::common::{apply_alias_retroactively, refresh_filtered_tasks, save_config};
use crate::model::{Task as TodoTask, extract_inline_aliases};
use iced::Task;
use iced::widget::operation;
use iced::widget::scrollable::RelativeOffset;

pub fn handle(app: &mut GuiApp, message: Message) -> Task<Message> {
    match message {
        Message::InputChanged(value) => {
            app.input_value = value;
            Task::none()
        }
        Message::DescriptionChanged(action) => {
            app.description_value.perform(action);
            Task::none()
        }
        Message::StartCreateChild(parent_uid) => {
            app.creating_child_of = Some(parent_uid.clone());
            app.selected_uid = Some(parent_uid.clone());

            // Auto-fill tags from parent
            let mut initial_input = String::new();

            // Directly access task to get categories
            if let Some((parent, _)) = app.store.get_task_mut(&parent_uid) {
                for cat in &parent.categories {
                    initial_input.push_str(&format!("#{} ", cat));
                }
            }

            app.input_value = initial_input;
            Task::none()
        }
        Message::SubmitTask => handle_submit(app),

        Message::EditTaskStart(index) => {
            if let Some(task) = app.tasks.get(index) {
                app.input_value = task.to_smart_string();
                app.description_value =
                    iced::widget::text_editor::Content::with_text(&task.description);
                app.editing_uid = Some(task.uid.clone());
                app.selected_uid = Some(task.uid.clone());
            }
            Task::none()
        }
        Message::CancelEdit => {
            app.input_value.clear();
            app.description_value = iced::widget::text_editor::Content::new();
            app.editing_uid = None;
            app.creating_child_of = None;
            Task::none()
        }

        Message::ToggleTask(index, _) => {
            if let Some(view_task) = app.tasks.get(index) {
                let uid = view_task.uid.clone();
                app.selected_uid = Some(uid.clone());
                if let Some(updated) = app.store.toggle_task(&uid) {
                    refresh_filtered_tasks(app);
                    if let Some(client) = &app.client {
                        return Task::perform(
                            async_toggle_wrapper(client.clone(), updated),
                            |res| Message::SyncToggleComplete(Box::new(res)),
                        );
                    }
                }
            }
            Task::none()
        }
        Message::DeleteTask(index) => {
            if let Some(view_task) = app.tasks.get(index)
                && let Some(deleted) = app.store.delete_task(&view_task.uid)
            {
                refresh_filtered_tasks(app);
                if let Some(client) = &app.client {
                    return Task::perform(
                        async_delete_wrapper(client.clone(), deleted),
                        Message::DeleteComplete,
                    );
                }
            }
            Task::none()
        }
        Message::ChangePriority(index, delta) => {
            if let Some(view_task) = app.tasks.get(index) {
                app.selected_uid = Some(view_task.uid.clone());
                if let Some(updated) = app.store.change_priority(&view_task.uid, delta) {
                    refresh_filtered_tasks(app);
                    if let Some(client) = &app.client {
                        return Task::perform(
                            async_update_wrapper(client.clone(), updated),
                            Message::SyncSaved,
                        );
                    }
                }
            }
            Task::none()
        }
        Message::SetTaskStatus(index, new_status) => {
            if let Some(view_task) = app.tasks.get(index) {
                app.selected_uid = Some(view_task.uid.clone());
                if let Some(updated) = app.store.set_status(&view_task.uid, new_status) {
                    refresh_filtered_tasks(app);
                    if let Some(client) = &app.client {
                        return Task::perform(
                            async_update_wrapper(client.clone(), updated),
                            Message::SyncSaved,
                        );
                    }
                }
            }
            Task::none()
        }
        // --- YANK / LINKING Handlers ---
        Message::YankTask(uid) => {
            app.yanked_uid = Some(uid);
            Task::none()
        }
        Message::ClearYank => {
            app.yanked_uid = None;
            Task::none()
        }
        Message::MakeChild(target_uid) => {
            if let Some(parent_uid) = &app.yanked_uid
                && let Some(updated) = app.store.set_parent(&target_uid, Some(parent_uid.clone()))
            {
                app.selected_uid = Some(target_uid);
                refresh_filtered_tasks(app);
                if let Some(client) = &app.client {
                    return Task::perform(
                        async_update_wrapper(client.clone(), updated),
                        Message::SyncSaved,
                    );
                }
            }
            Task::none()
        }
        Message::RemoveParent(child_uid) => {
            if let Some(updated) = app.store.set_parent(&child_uid, None) {
                app.selected_uid = Some(child_uid);
                refresh_filtered_tasks(app);
                if let Some(client) = &app.client {
                    return Task::perform(
                        async_update_wrapper(client.clone(), updated),
                        Message::SyncSaved,
                    );
                }
            }
            Task::none()
        }
        Message::RemoveDependency(task_uid, dep_uid) => {
            if let Some(updated) = app.store.remove_dependency(&task_uid, &dep_uid) {
                app.selected_uid = Some(task_uid);
                refresh_filtered_tasks(app);
                if let Some(client) = &app.client {
                    return Task::perform(
                        async_update_wrapper(client.clone(), updated),
                        Message::SyncSaved,
                    );
                }
            }
            Task::none()
        }
        Message::AddDependency(target_uid) => {
            if let Some(blocker_uid) = &app.yanked_uid
                && let Some(updated) = app.store.add_dependency(&target_uid, blocker_uid.clone())
            {
                app.selected_uid = Some(target_uid);
                refresh_filtered_tasks(app);
                if let Some(client) = &app.client {
                    return Task::perform(
                        async_update_wrapper(client.clone(), updated),
                        Message::SyncSaved,
                    );
                }
            }
            Task::none()
        }
        Message::MoveTask(task_uid, target_href) => {
            if let Some(updated) = app.store.move_task(&task_uid, target_href.clone()) {
                app.selected_uid = Some(task_uid);
                refresh_filtered_tasks(app);
                if let Some(client) = &app.client {
                    return Task::perform(
                        async_move_wrapper(client.clone(), updated, target_href),
                        Message::TaskMoved,
                    );
                }
            }
            Task::none()
        }
        Message::MigrateLocalTo(target_href) => {
            if let Some(local_tasks) = app.store.calendars.get(crate::storage::LOCAL_CALENDAR_HREF)
            {
                let tasks_to_move = local_tasks.clone();
                if tasks_to_move.is_empty() {
                    return Task::none();
                }
                app.loading = true;
                if let Some(client) = &app.client {
                    return Task::perform(
                        async_migrate_wrapper(client.clone(), tasks_to_move, target_href),
                        Message::MigrationComplete,
                    );
                }
            }
            Task::none()
        }
        _ => Task::none(),
    }
}

fn handle_submit(app: &mut GuiApp) -> Task<Message> {
    if app.input_value.is_empty() {
        return Task::none();
    }

    // --- Parse inline alias definitions (#key=tag1,tag2) ---
    let (clean_input, new_aliases) = extract_inline_aliases(&app.input_value);

    let mut retroactive_sync_batch = Vec::new();

    if !new_aliases.is_empty() {
        // Register new aliases
        for (key, tags) in new_aliases {
            app.tag_aliases.insert(key.clone(), tags.clone());

            // Queue retroactive application
            if let Some(task_cmd) = apply_alias_retroactively(app, &key, &tags) {
                retroactive_sync_batch.push(task_cmd);
            }
        }
        save_config(app);
    }

    if clean_input.starts_with('#')
        && !clean_input.trim().contains(' ')
        && app.editing_uid.is_none()
    {
        // If we just parsed aliases (e.g. #a=#b), clean_input might be "#a".
        // In that case, we treat it as a definition, not a jump request.
        let was_alias_definition = app.input_value.contains('=');

        if !was_alias_definition {
            let tag = clean_input.trim().trim_start_matches('#').to_string();
            if !tag.is_empty() {
                app.sidebar_mode = SidebarMode::Categories;
                app.selected_categories.clear();
                app.selected_categories.insert(tag);
                app.input_value.clear();
                refresh_filtered_tasks(app);

                if !retroactive_sync_batch.is_empty() {
                    return Task::batch(retroactive_sync_batch);
                }
                return Task::none();
            }
        } else {
            // It was a definition. Just clear input and run pending syncs.
            app.input_value.clear();
            refresh_filtered_tasks(app);
            if !retroactive_sync_batch.is_empty() {
                return Task::batch(retroactive_sync_batch);
            }
            return Task::none();
        }
    }

    if let Some(edit_uid) = &app.editing_uid {
        if let Some((task, _)) = app.store.get_task_mut(edit_uid) {
            task.apply_smart_input(&clean_input, &app.tag_aliases);
            task.description = app.description_value.text();
            let task_copy = task.clone();

            app.input_value.clear();
            app.description_value = iced::widget::text_editor::Content::new();
            app.editing_uid = None;
            app.selected_uid = Some(task_copy.uid.clone());

            refresh_filtered_tasks(app);
            if let Some(client) = &app.client {
                let save_cmd = Task::perform(
                    async_update_wrapper(client.clone(), task_copy),
                    Message::SyncSaved,
                );
                retroactive_sync_batch.push(save_cmd);
                return Task::batch(retroactive_sync_batch);
            }
        }
    } else if !clean_input.is_empty() {
        let mut new_task = TodoTask::new(&clean_input, &app.tag_aliases);
        if let Some(parent) = &app.creating_child_of {
            new_task.parent_uid = Some(parent.clone());
            app.creating_child_of = None;
        }

        let target_href = app
            .active_cal_href
            .clone()
            .or_else(|| app.calendars.first().map(|c| c.href.clone()))
            .unwrap_or_default();

        if !target_href.is_empty() {
            new_task.calendar_href = target_href.clone();

            // Fix: Use add_task to maintain index
            app.store.add_task(new_task.clone());

            app.selected_uid = Some(new_task.uid.clone());
            refresh_filtered_tasks(app);
            app.input_value.clear();

            let len = app.tasks.len().max(1) as f32;
            let idx = app
                .tasks
                .iter()
                .position(|t| t.uid == new_task.uid)
                .unwrap_or(0) as f32;
            let scroll_cmd = operation::snap_to(
                app.scrollable_id.clone(),
                RelativeOffset {
                    x: 0.0,
                    y: idx / len,
                },
            );

            if let Some(client) = &app.client {
                let create_cmd = Task::perform(
                    async_create_wrapper(client.clone(), new_task),
                    Message::SyncSaved,
                );

                retroactive_sync_batch.push(create_cmd);
                retroactive_sync_batch.push(scroll_cmd);

                return Task::batch(retroactive_sync_batch);
            }
        }
    }

    if !retroactive_sync_batch.is_empty() {
        return Task::batch(retroactive_sync_batch);
    }
    Task::none()
}
