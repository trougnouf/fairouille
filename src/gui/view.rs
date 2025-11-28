use crate::gui::message::Message;
use crate::gui::state::{AppState, GuiApp, SidebarMode};
use crate::model::Task as TodoTask;
use crate::store::UNCATEGORIZED_ID;

use iced::widget::{
    Rule, button, checkbox, column, container, horizontal_space, row, scrollable, text, text_input,
};
use iced::{Background, Color, Element, Length, Theme};

#[derive(Debug, Clone, PartialEq, Eq)]
struct DurationOpt(Option<u32>, String);

impl std::fmt::Display for DurationOpt {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.1)
    }
}

// Formatting Helper
fn format_mins(m: u32) -> String {
    if m >= 525600 {
        format!("{}y", m / 525600)
    } else if m >= 43200 {
        format!("{}mo", m / 43200)
    } else if m >= 10080 {
        format!("{}w", m / 10080)
    } else if m >= 1440 {
        format!("{}d", m / 1440)
    } else if m >= 60 {
        format!("{}h", m / 60)
    } else {
        format!("{}m", m)
    }
}

pub fn root_view(app: &GuiApp) -> Element<'_, Message> {
    match app.state {
        AppState::Loading => container(text("Loading...").size(30))
            .width(Length::Fill)
            .height(Length::Fill)
            .center_x(Length::Fill)
            .center_y(Length::Fill)
            .into(),
        AppState::Onboarding | AppState::Settings => view_settings(app),
        AppState::Active => {
            let layout = row![
                view_sidebar(app),
                Rule::vertical(1),
                container(view_main_content(app))
                    .width(Length::Fill)
                    .center_x(Length::Fill)
            ];
            container(layout)
                .width(Length::Fill)
                .height(Length::Fill)
                .into()
        }
    }
}

// --- SIDEBAR COMPONENT ---

fn view_sidebar(app: &GuiApp) -> Element<'_, Message> {
    // 1. Tab Switcher
    let btn_cals = button(
        container(text("Calendars").size(14))
            .width(Length::Fill)
            .align_x(iced::alignment::Horizontal::Center),
    )
    .padding(5)
    .width(Length::Fill)
    .style(if app.sidebar_mode == SidebarMode::Calendars {
        button::primary
    } else {
        button::secondary
    })
    .on_press(Message::SidebarModeChanged(SidebarMode::Calendars));

    let btn_tags = button(
        container(text("Tags").size(14))
            .width(Length::Fill)
            .align_x(iced::alignment::Horizontal::Center),
    )
    .padding(5)
    .width(Length::Fill)
    .style(if app.sidebar_mode == SidebarMode::Categories {
        button::primary
    } else {
        button::secondary
    })
    .on_press(Message::SidebarModeChanged(SidebarMode::Categories));

    let tabs = row![btn_cals, btn_tags].spacing(5);

    // 2. Content based on Tab
    let content = match app.sidebar_mode {
        SidebarMode::Calendars => view_sidebar_calendars(app),
        SidebarMode::Categories => view_sidebar_categories(app),
    };

    // 3. Footer (Settings)
    let settings_btn = button(row![text("Settings").size(16)].align_y(iced::Alignment::Center))
        .padding(10)
        .width(Length::Fill)
        .style(button::secondary)
        .on_press(Message::OpenSettings);

    let sidebar_inner = column![tabs, scrollable(content).height(Length::Fill), settings_btn]
        .spacing(10)
        .padding(10);

    container(sidebar_inner)
        .width(200)
        .height(Length::Fill)
        .style(|theme: &Theme| {
            let palette = theme.extended_palette();
            container::Style {
                background: Some(Background::Color(palette.background.weak.color)),
                ..Default::default()
            }
        })
        .into()
}

fn view_sidebar_calendars(app: &GuiApp) -> Element<'_, Message> {
    column(
        app.calendars
            .iter()
            .filter(|c| !app.hidden_calendars.contains(&c.href))
            .map(|cal| {
                let is_active = app.active_cal_href.as_ref() == Some(&cal.href);
                let btn = button(text(&cal.name).size(16))
                    .padding(10)
                    .width(Length::Fill)
                    .on_press(Message::SelectCalendar(cal.href.clone()));

                if is_active {
                    btn.style(button::primary)
                } else {
                    btn.style(button::secondary)
                }
                .into()
            })
            .collect::<Vec<_>>(),
    )
    .spacing(10)
    .width(Length::Fill)
    .into()
}

fn view_sidebar_categories(app: &GuiApp) -> Element<'_, Message> {
    // 1. Existing Category Logic
    let all_cats = app.store.get_all_categories(
        app.hide_completed,
        app.hide_fully_completed_tags,
        &app.selected_categories,
        &app.hidden_calendars,
    );

    let logic_text = if app.match_all_categories {
        "Match: AND"
    } else {
        "Match: OR"
    };
    let logic_btn = button(text(logic_text).size(12))
        .style(button::secondary)
        .padding(5)
        .on_press(Message::CategoryMatchModeChanged(!app.match_all_categories));

    let header = row![
        text("Filter Tags")
            .size(14)
            .color(Color::from_rgb(0.7, 0.7, 0.7)),
        horizontal_space(),
        logic_btn
    ]
    .align_y(iced::Alignment::Center)
    .padding(iced::Padding {
        right: 15.0,
        ..Default::default()
    });

    let tags_list: Element<'_, Message> = if all_cats.is_empty() {
        column![
            header,
            text("No tags found")
                .size(14)
                .color(Color::from_rgb(0.5, 0.5, 0.5))
        ]
        .spacing(10)
        .into()
    } else {
        let list = column(
            all_cats
                .into_iter()
                .map(|cat| {
                    let is_selected = app.selected_categories.contains(&cat);
                    let cat_clone = cat.clone();
                    let display_name = if cat == UNCATEGORIZED_ID {
                        "Uncategorized".to_string()
                    } else {
                        format!("#{}", cat)
                    };

                    checkbox(display_name, is_selected)
                        .size(18)
                        .text_size(16)
                        .on_toggle(move |_| Message::CategoryToggled(cat_clone.clone()))
                        .into()
                })
                .collect::<Vec<_>>(),
        )
        .spacing(5);
        column![header, list].spacing(10).into()
    };

    // 2. Dynamic Duration Filter Section
    let mut dur_set = std::collections::HashSet::new();
    // Scan ALL tasks
    for tasks in app.store.calendars.values() {
        for t in tasks {
            if let Some(d) = t.estimated_duration {
                dur_set.insert(d);
            }
        }
    }
    let mut sorted_durs: Vec<u32> = dur_set.into_iter().collect();
    sorted_durs.sort();

    // Build Options
    let mut opts = vec![DurationOpt(None, "Any".to_string())];
    for d in sorted_durs {
        opts.push(DurationOpt(Some(d), format_mins(d)));
    }

    // Determine Current Selection (Robust matching)
    let current_min = opts
        .iter()
        .find(|o| o.0 == app.filter_min_duration)
        .cloned()
        .unwrap_or_else(|| opts[0].clone());

    let current_max = opts
        .iter()
        .find(|o| o.0 == app.filter_max_duration)
        .cloned()
        .unwrap_or_else(|| opts[0].clone());

    let dur_filters = column![
        Rule::horizontal(1),
        text("Filter Duration")
            .size(14)
            .color(Color::from_rgb(0.7, 0.7, 0.7)),
        row![
            text("Min:").size(12).width(30),
            iced::widget::pick_list(opts.clone(), Some(current_min), |o| {
                Message::SetMinDuration(o.0)
            })
            .text_size(12)
            .padding(5)
            .width(Length::Fill)
        ]
        .spacing(5)
        .align_y(iced::Alignment::Center),
        row![
            text("Max:").size(12).width(30),
            iced::widget::pick_list(opts, Some(current_max), |o| Message::SetMaxDuration(o.0))
                .text_size(12)
                .padding(5)
                .width(Length::Fill)
        ]
        .spacing(5)
        .align_y(iced::Alignment::Center),
        checkbox("Include Unset", app.filter_include_unset_duration)
            .text_size(12)
            .size(16)
            .on_toggle(Message::ToggleIncludeUnsetDuration)
    ]
    .spacing(8)
    .padding(iced::Padding {
        top: 10.0,
        ..Default::default()
    });

    // Combine Tag List + Duration Filters
    column![tags_list, dur_filters].spacing(10).into()
}

// --- MAIN CONTENT COMPONENT ---

fn view_main_content(app: &GuiApp) -> Element<'_, Message> {
    let title_text = if app.loading {
        "Loading...".to_string()
    } else if app.active_cal_href.is_none() {
        if app.selected_categories.is_empty() {
            "All Tasks".to_string()
        } else {
            format!("Tasks ({})", app.tasks.len())
        }
    } else {
        app.calendars
            .iter()
            .find(|c| Some(&c.href) == app.active_cal_href.as_ref())
            .map(|c| c.name.clone())
            .unwrap_or("Calendar".to_string())
    };

    let search_input = text_input("Search...", &app.search_value)
        .on_input(Message::SearchChanged)
        .padding(5)
        .size(16);

    let header = row![
        text(title_text).size(40),
        horizontal_space(),
        search_input.width(200)
    ]
    .align_y(iced::Alignment::Center);

    let input_area = view_input_area(app);

    let tasks_view = column(
        app.tasks
            .iter()
            .enumerate()
            .map(|(real_index, task)| view_task_row(app, real_index, task))
            .collect::<Vec<_>>(),
    )
    .spacing(2);

    column![
        header,
        input_area,
        scrollable(tasks_view).height(Length::Fill)
    ]
    .spacing(20)
    .padding(20)
    .max_width(800)
    .into()
}

fn view_input_area(app: &GuiApp) -> Element<'_, Message> {
    let input_placeholder = if app.editing_uid.is_some() {
        "Edit Title..."
    } else {
        "Add task (Buy cat food !1 @weekly #groceries ~30m)..."
    };

    // 1. Main Text Input
    let input_title = text_input(input_placeholder, &app.input_value)
        .on_input(Message::InputChanged)
        .on_submit(Message::SubmitTask)
        .padding(10)
        .size(20);

    // 3. Layout Construction
    if app.editing_uid.is_some() {
        let input_desc = text_input("Notes...", &app.description_value)
            .on_input(Message::DescriptionChanged)
            .on_submit(Message::SubmitTask)
            .padding(10)
            .size(16);

        let cancel_btn = button(text("Cancel").size(16))
            .style(button::secondary)
            .on_press(Message::CancelEdit);

        let save_btn = button(text("Save").size(16))
            .style(button::primary)
            .on_press(Message::SubmitTask);

        // 1. Top Bar: Label + Save/Cancel (Always clean, never blocked)
        let top_bar = row![
            text("Editing")
                .size(14)
                .color(Color::from_rgb(0.7, 0.7, 1.0)),
            horizontal_space(),
            cancel_btn,
            save_btn
        ]
        .align_y(iced::Alignment::Center)
        .spacing(10);

        // 2. Move Section (Conditional, Filtered, Scrollable)
        let mut move_element: Element<'_, Message> = row![].into();

        if let Some(edit_uid) = &app.editing_uid {
            if let Some(task) = app.tasks.iter().find(|t| t.uid == *edit_uid) {
                // Filter: Exclude current calendar AND hidden calendars
                let targets: Vec<_> = app
                    .calendars
                    .iter()
                    .filter(|c| {
                        c.href != task.calendar_href && !app.hidden_calendars.contains(&c.href)
                    })
                    .collect();

                if !targets.is_empty() {
                    let label = text("Move to:")
                        .size(12)
                        .color(Color::from_rgb(0.6, 0.6, 0.6));

                    let mut btn_row = row![].spacing(5);
                    for cal in targets {
                        btn_row = btn_row.push(
                            button(text(&cal.name).size(12))
                                .style(button::secondary)
                                .padding(5)
                                .on_press(Message::MoveTask(task.uid.clone(), cal.href.clone())),
                        );
                    }

                    move_element = row![
                        label,
                        scrollable(btn_row).height(30) // Constrain height to prevent layout jumps
                    ]
                    .spacing(10)
                    .align_y(iced::Alignment::Center)
                    .into();
                }
            }
        }

        // 3. Assemble Layout
        column![
            top_bar,
            input_title,
            input_desc,
            move_element // Placed at bottom of edit area, or swap with input_desc if preferred
        ]
        .spacing(10)
        .into()
    } else {
        column![input_title,].spacing(5).into()
    }
}

fn view_task_row<'a>(app: &'a GuiApp, index: usize, task: &'a TodoTask) -> Element<'a, Message> {
    // 1. Check Blocked Status
    let is_blocked = app.store.is_blocked(task);

    let color = if is_blocked {
        Color::from_rgb(0.5, 0.5, 0.5)
    } else {
        match task.priority {
            1..=4 => Color::from_rgb(0.8, 0.2, 0.2),
            5 => Color::from_rgb(0.8, 0.8, 0.2),
            _ => Color::WHITE,
        }
    };

    let show_indent = app.active_cal_href.is_some() && app.search_value.is_empty();
    let indent_size = if show_indent { task.depth * 20 } else { 0 };
    let indent = horizontal_space().width(Length::Fixed(indent_size as f32));

    // 2. Title Row (Just Summary)
    let title_row = row![
        text(&task.summary)
            .size(20)
            .color(color)
            .width(Length::Fill)
    ]
    .spacing(10);

    // 3. Tags / Meta Row (Blocked Badge + Categories + Recurrence)
    let mut tags_row: iced::widget::Row<'_, Message> = row![].spacing(5);

    // [Blocked] Badge moved here
    if is_blocked {
        tags_row = tags_row.push(
            container(text("[Blocked]").size(12).color(Color::WHITE))
                .style(|_| container::Style {
                    background: Some(Color::from_rgb(0.8, 0.2, 0.2).into()), // Red background for visibility
                    border: iced::Border {
                        radius: 4.0.into(),
                        ..Default::default()
                    },
                    ..Default::default()
                })
                .padding(3),
        );
    }

    for cat in &task.categories {
        tags_row = tags_row.push(
            container(text(format!("#{}", cat)).size(12).color(Color::BLACK))
                .style(|_| container::Style {
                    background: Some(Color::from_rgb(0.6, 0.8, 1.0).into()),
                    border: iced::Border {
                        radius: 4.0.into(),
                        ..Default::default()
                    },
                    ..Default::default()
                })
                .padding(3),
        );
    }

    if task.rrule.is_some() {
        tags_row = tags_row.push(text("(R)").size(14).color(Color::from_rgb(0.6, 0.6, 1.0)));
    }

    if let Some(mins) = task.estimated_duration {
        let label = if mins >= 525600 {
            format!("{}y", mins / 525600)
        } else if mins >= 43200 {
            format!("{}mo", mins / 43200)
        } else if mins >= 10080 {
            format!("{}w", mins / 10080)
        } else if mins >= 1440 {
            format!("{}d", mins / 1440)
        } else if mins >= 60 {
            format!("{}h", mins / 60)
        } else {
            format!("{}m", mins)
        };

        tags_row = tags_row.push(
            container(text(label).size(10).color(Color::WHITE))
                .style(|_| container::Style {
                    background: Some(Color::from_rgb(0.5, 0.5, 0.5).into()),
                    border: iced::Border {
                        radius: 4.0.into(),
                        ..Default::default()
                    },
                    ..Default::default()
                })
                .padding(3),
        );
    }

    let date_text = match task.due {
        Some(d) => text(d.format("%Y-%m-%d").to_string())
            .size(14)
            .color(Color::from_rgb(0.5, 0.5, 0.5)),
        None => text(""),
    };

    // 4. Info Button
    let has_desc = !task.description.is_empty();
    let has_deps = !task.dependencies.is_empty();
    let is_expanded = app.expanded_tasks.contains(&task.uid);

    let info_btn = if has_desc || has_deps {
        button(text("i").size(12))
            .style(if is_expanded {
                button::primary
            } else {
                button::secondary
            })
            .padding(5)
            .width(Length::Fixed(25.0))
            .on_press(Message::ToggleDetails(task.uid.clone()))
    } else {
        button(text("").size(12))
            .style(button::text)
            .padding(5)
            .width(Length::Fixed(25.0))
    };

    // 5. Actions
    let mut actions = row![info_btn].spacing(5);

    if let Some(yanked) = &app.yanked_uid {
        if *yanked != task.uid {
            actions = actions.push(
                button(text("Block").size(14))
                    .style(button::secondary)
                    .padding(5)
                    .on_press(Message::AddDependency(task.uid.clone())),
            );
            actions = actions.push(
                button(text("Child").size(14))
                    .style(button::secondary)
                    .padding(5)
                    .on_press(Message::MakeChild(task.uid.clone())),
            );
        } else {
            actions = actions.push(
                button(text("Unlink").size(14))
                    .style(button::primary)
                    .padding(5)
                    .on_press(Message::ClearYank),
            );
        }
    } else {
        actions = actions.push(
            button(text("Link").size(14))
                .style(button::secondary)
                .padding(5)
                .on_press(Message::YankTask(task.uid.clone())),
        );
    }

    let btn_style = button::secondary;

    // Status Controls
    if task.status != crate::model::TaskStatus::Completed
        && task.status != crate::model::TaskStatus::Cancelled
    {
        let (icon, msg_status) = if task.status == crate::model::TaskStatus::InProcess {
            ("||", crate::model::TaskStatus::NeedsAction)
        } else {
            (">", crate::model::TaskStatus::InProcess)
        };
        actions = actions.push(
            button(text(icon).size(14))
                .style(btn_style)
                .padding(5)
                .on_press(Message::SetTaskStatus(index, msg_status)),
        );
    }

    actions = actions.push(
        button(text("+").size(14))
            .style(btn_style)
            .padding(5)
            .on_press(Message::ChangePriority(index, 1)),
    );
    actions = actions.push(
        button(text("-").size(14))
            .style(btn_style)
            .padding(5)
            .on_press(Message::ChangePriority(index, -1)),
    );
    actions = actions.push(
        button(text("Edit").size(14))
            .style(btn_style)
            .padding(5)
            .on_press(Message::EditTaskStart(index)),
    );

    // Cancel Button (Moved here)
    if task.status != crate::model::TaskStatus::Completed
        && task.status != crate::model::TaskStatus::Cancelled
    {
        actions = actions.push(
            button(text("ø").size(14))
                .style(button::danger)
                .padding(5)
                .on_press(Message::SetTaskStatus(
                    index,
                    crate::model::TaskStatus::Cancelled,
                )),
        );
    }

    actions = actions.push(
        button(text("Del").size(14))
            .style(button::danger)
            .padding(5)
            .on_press(Message::DeleteTask(index)),
    );

    // 6. Construct Main Row
    // Cast tags_row to Element to avoid type inference issues
    let tags_element: Element<'a, Message> = tags_row.into();

    // --- CUSTOM MULTI-STATE CHECKBOX ---
    // We define the look (Icon + Background Color) based on status
    let (icon_char, bg_color, border_color) = match task.status {
        crate::model::TaskStatus::InProcess => (
            ">",
            // Toned down / Muted Green
            Color::from_rgb(0.6, 0.8, 0.6),
            Color::from_rgb(0.4, 0.5, 0.4),
        ),
        crate::model::TaskStatus::Cancelled => (
            "X",
            Color::from_rgb(0.3, 0.2, 0.2),
            Color::from_rgb(0.5, 0.4, 0.4),
        ),
        crate::model::TaskStatus::Completed => (
            "V",
            // The "Pretty" Bright Green
            Color::from_rgb(0.0, 0.6, 0.0),
            Color::from_rgb(0.0, 0.8, 0.0),
        ),
        crate::model::TaskStatus::NeedsAction => {
            (" ", Color::TRANSPARENT, Color::from_rgb(0.5, 0.5, 0.5))
        }
    };

    // We use a Button fixed to 24x24 to act as a Checkbox
    let status_btn = button(
        container(
            text(icon_char)
                .size(12) // Reduced size (was 14)
                .color(Color::WHITE),
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .align_x(iced::alignment::Horizontal::Center)
        .align_y(iced::alignment::Vertical::Center),
    )
    .width(Length::Fixed(24.0))
    .height(Length::Fixed(24.0))
    .padding(0)
    .on_press(Message::ToggleTask(index, true)) // Logic handles the toggle
    .style(move |_theme, status| {
        // Define how the "Box" looks
        let base_active = button::Style {
            background: Some(bg_color.into()),
            text_color: Color::WHITE,
            border: iced::Border {
                color: border_color,
                width: 1.0,
                radius: 4.0.into(), // Rounded corners like a standard checkbox
            },
            ..button::Style::default()
        };

        match status {
            iced::widget::button::Status::Hovered => button::Style {
                // Slight highlight on hover
                border: iced::Border {
                    color: Color::WHITE,
                    width: 1.0,
                    radius: 4.0.into(),
                },
                ..base_active
            },
            _ => base_active,
        }
    });
    let row_main = row![
        indent,
        status_btn,
        column![
            title_row,
            // Show tags line if there are tags OR recurrence OR task is blocked
            if !task.categories.is_empty() || task.rrule.is_some() || is_blocked {
                tags_element
            } else {
                row![].into()
            }
        ]
        .width(Length::Fill)
        .spacing(2),
        date_text,
        actions
    ]
    .spacing(15)
    .align_y(iced::Alignment::Center);

    let padded_row = container(row_main).padding(iced::Padding {
        top: 5.0,
        right: 15.0,
        bottom: 5.0,
        left: 5.0,
    });

    if is_expanded {
        let mut details_col = column![].spacing(5);

        // 1. Description
        if !task.description.is_empty() {
            details_col = details_col.push(
                text(&task.description)
                    .size(14)
                    .color(Color::from_rgb(0.7, 0.7, 0.7)),
            );
        }

        // 2. Parent (NEW: Show parent and allow detaching)
        if let Some(p_uid) = &task.parent_uid {
            let p_name = app
                .store
                .get_summary(p_uid)
                .unwrap_or_else(|| "Unknown Parent".to_string());
            let row = row![
                text("Parent:")
                    .size(12)
                    .color(Color::from_rgb(0.4, 0.8, 0.4)),
                text(p_name).size(12), // Pass ownership (remove &)
                button(text("x").size(10))
                    .style(button::danger)
                    .padding(2)
                    .on_press(Message::RemoveParent(task.uid.clone()))
            ]
            .spacing(5)
            .align_y(iced::Alignment::Center);
            details_col = details_col.push(row);
        }

        // 3. Dependencies (Updated with remove button)
        if !task.dependencies.is_empty() {
            details_col = details_col.push(
                text("[Blocked By]:")
                    .size(12)
                    .color(Color::from_rgb(0.8, 0.4, 0.4)),
            );
            for dep_uid in &task.dependencies {
                let name = app
                    .store
                    .get_summary(dep_uid)
                    .unwrap_or_else(|| "Unknown Task".to_string());
                let is_done = app.store.is_task_done(dep_uid).unwrap_or(false);
                let check = if is_done { "[x]" } else { "[ ]" };

                let dep_row = row![
                    text(format!("{} {}", check, name))
                        .size(12)
                        .color(Color::from_rgb(0.6, 0.6, 0.6)),
                    button(text("x").size(10))
                        .style(button::danger)
                        .padding(2)
                        .on_press(Message::RemoveDependency(task.uid.clone(), dep_uid.clone()))
                ]
                .spacing(5)
                .align_y(iced::Alignment::Center);

                details_col = details_col.push(dep_row);
            }
        }

        // Only show if we have multiple calendars
        if app.calendars.len() > 1 {
            let current_cal_href = task.calendar_href.clone();

            // Build a list of target calendars (excluding the current one)
            let targets: Vec<_> = app
                .calendars
                .iter()
                .filter(|c| c.href != current_cal_href)
                .collect();

            let move_label = text("Move to:")
                .size(12)
                .color(Color::from_rgb(0.5, 0.5, 0.5));

            // We use a Row of buttons for targets (simpler than PickList for per-row state)
            // Or a PickList if you prefer. A row of small buttons is often faster.
            let mut move_row = row![move_label].spacing(5).align_y(iced::Alignment::Center);

            for cal in targets {
                move_row = move_row.push(
                    button(text(&cal.name).size(10))
                        .style(button::secondary)
                        .padding(3)
                        .on_press(Message::MoveTask(task.uid.clone(), cal.href.clone())),
                );
            }

            details_col = details_col.push(move_row);
        }

        let desc_row = row![
            horizontal_space().width(Length::Fixed(indent_size as f32 + 30.0)),
            details_col
        ];
        container(column![padded_row, desc_row].spacing(5))
            .padding(5)
            .into()
    } else {
        padded_row.into()
    }
}

fn view_settings(app: &GuiApp) -> Element<'_, Message> {
    let is_settings = matches!(app.state, AppState::Settings);
    let title = text(if is_settings {
        "Settings"
    } else {
        "Welcome to Cfait"
    })
    .size(40);
    let error = if let Some(e) = &app.error_msg {
        text(e).color(Color::from_rgb(1.0, 0.0, 0.0))
    } else {
        text("")
    };

    let cal_names: Vec<String> = app.calendars.iter().map(|c| c.name.clone()).collect();
    let picker: Element<_> = if !cal_names.is_empty() && is_settings {
        column![
            text("Default Calendar:"),
            iced::widget::pick_list(
                cal_names,
                app.ob_default_cal.clone(),
                Message::ObDefaultCalChanged
            )
            .width(Length::Fill)
            .padding(10)
        ]
        .spacing(5)
        .into()
    } else {
        horizontal_space().width(0).into()
    };

    let prefs: Element<'_, Message> = if is_settings {
        std::convert::Into::<Element<'_, Message>>::into(container(
            column![
                std::convert::Into::<Element<'_, Message>>::into(
                    checkbox("Hide Completed Tasks (Everywhere)", app.hide_completed)
                        .on_toggle(Message::ToggleHideCompleted),
                ),
                // Conditional checkbox: only visible when 'Hide Completed Tasks (Everywhere)' is off
                if !app.hide_completed {
                    std::convert::Into::<Element<'_, Message>>::into(
                        checkbox(
                            "Hide Tags containing ONLY completed tasks",
                            app.hide_fully_completed_tags,
                        )
                        .on_toggle(Message::ToggleHideFullyCompletedTags),
                    )
                } else {
                    // Placeholder to keep spacing
                    std::convert::Into::<Element<'_, Message>>::into(horizontal_space().width(0))
                },
            ]
            .spacing(10),
        ))
    } else {
        std::convert::Into::<Element<'_, Message>>::into(horizontal_space().width(0))
    };

    let sorting_ui: Element<_> = if is_settings {
        column![
            text("Sorting Priority Cutoff (Months):"),
            text("(Tasks due within this range are shown first. Blank = All timed first)")
                .size(12)
                .color(Color::from_rgb(0.6, 0.6, 0.6)),
            text_input("6", &app.ob_sort_months_input)
                .on_input(Message::ObSortMonthsChanged)
                .padding(10)
                .width(Length::Fixed(100.0))
        ]
        .spacing(5)
        .into()
    } else {
        horizontal_space().width(0).into()
    };

    // Alias Section
    let aliases_ui: Element<_> = if is_settings {
        let mut list_col = column![text("Tag Aliases").size(20)].spacing(10);

        // Existing Aliases List
        for (key, vals) in &app.tag_aliases {
            let val_str = vals.join(", ");
            let row_item = row![
                text(format!("#{}", key)).width(Length::FillPortion(1)),
                text("->").width(Length::Fixed(20.0)),
                text(val_str).width(Length::FillPortion(2)),
                button(text("X").size(12))
                    .style(button::danger)
                    .padding(5)
                    .on_press(Message::RemoveAlias(key.clone()))
            ]
            .spacing(10)
            .align_y(iced::Alignment::Center);
            list_col = list_col.push(row_item);
        }

        // Add New Alias Form
        let input_row = row![
            text_input("Alias (#cfait)", &app.alias_input_key)
                .on_input(Message::AliasKeyInput)
                .padding(5)
                .width(Length::FillPortion(1)),
            text_input("Tags (dev, rust)", &app.alias_input_values)
                .on_input(Message::AliasValueInput)
                .padding(5)
                .width(Length::FillPortion(2)),
            button("Add").padding(5).on_press(Message::AddAlias)
        ]
        .spacing(10);

        let area = container(column![list_col, Rule::horizontal(1), input_row].spacing(15))
            .padding(10)
            .style(|_| container::Style {
                border: iced::Border {
                    radius: 4.0.into(),
                    width: 1.0,
                    color: Color::from_rgb(0.3, 0.3, 0.3),
                },
                ..Default::default()
            });

        area.into()
    } else {
        horizontal_space().width(0).into()
    };

    let cal_mgmt_ui: Element<_> = if is_settings && !app.calendars.is_empty() {
        let mut col = column![text("Manage Calendars").size(20)].spacing(10);

        for cal in &app.calendars {
            let is_visible = !app.hidden_calendars.contains(&cal.href);
            col = col.push(
                checkbox(&cal.name, is_visible)
                    .on_toggle(move |v| Message::ToggleCalendarVisibility(cal.href.clone(), v)),
            );
        }

        container(col)
            .padding(10)
            .style(|_| container::Style {
                border: iced::Border {
                    radius: 4.0.into(),
                    width: 1.0,
                    color: Color::from_rgb(0.3, 0.3, 0.3),
                },
                ..Default::default()
            })
            .into()
    } else {
        horizontal_space().width(0).into()
    };

    let mut buttons = row![].spacing(10);
    if is_settings {
        buttons = buttons.push(
            button("Cancel")
                .padding(10)
                .style(button::secondary)
                .on_press(Message::CancelSettings),
        );
    }
    buttons = buttons.push(
        button(if is_settings {
            "Save & Connect"
        } else {
            "Connect"
        })
        .padding(10)
        .on_press(Message::ObSubmit),
    );
    let insecure_check = checkbox("Allow Insecure SSL (e.g. self-signed)", app.ob_insecure)
        .on_toggle(Message::ObInsecureToggled)
        .size(16)
        .text_size(14);
    let form = column![
        text("CalDAV Server URL:"),
        text_input("https://...", &app.ob_url)
            .on_input(Message::ObUrlChanged)
            .padding(10),
        text("Username:"),
        text_input("User", &app.ob_user)
            .on_input(Message::ObUserChanged)
            .padding(10),
        text("Password:"),
        text_input("Password", &app.ob_pass)
            .on_input(Message::ObPassChanged)
            .secure(true)
            .padding(10),
        insecure_check,
        picker,
        prefs,
        sorting_ui,
        aliases_ui,
        cal_mgmt_ui,
        buttons
    ]
    .spacing(15)
    .max_width(500); // Increased width for alias inputs

    let content = column![title, error, form]
        .spacing(20)
        .align_x(iced::Alignment::Center);

    // Wrap in scrollable so buttons are accessible on small screens
    container(scrollable(
        container(content)
            .width(Length::Fill)
            .padding(20)
            .center_x(Length::Fill),
    ))
    .width(Length::Fill)
    .height(Length::Fill)
    .into()
}
