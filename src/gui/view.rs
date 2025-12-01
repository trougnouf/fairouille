use crate::gui::icon;
use crate::gui::message::Message;
use crate::gui::state::{AppState, GuiApp, SidebarMode};
use crate::model::Task as TodoTask;
use crate::storage::LOCAL_CALENDAR_HREF;
use crate::store::UNCATEGORIZED_ID;

use iced::widget::{
    Rule, button, checkbox, column, container, horizontal_space, row, scrollable, text, text_input,
    toggler,
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
    // 1. Calculate "Select All" state
    let are_all_visible = app
        .calendars
        .iter()
        .filter(|c| !app.disabled_calendars.contains(&c.href))
        .all(|c| !app.hidden_calendars.contains(&c.href));

    let toggle_all = toggler(are_all_visible)
        .label("Show All")
        .text_size(12) // Tiny bit smaller
        .text_alignment(iced::alignment::Horizontal::Left)
        .spacing(10)
        .width(Length::Fill)
        .on_toggle(Message::ToggleAllCalendars);

    // Wrap in container for padding
    let toggle_container = container(toggle_all).padding(5);

    let list = column(
        app.calendars
            .iter()
            .filter(|c| !app.disabled_calendars.contains(&c.href)) // Filter directly here
            .map(|cal| {
                let is_visible = !app.hidden_calendars.contains(&cal.href);
                let is_target = app.active_cal_href.as_ref() == Some(&cal.href);

                let check = checkbox("", is_visible)
                    .on_toggle(move |v| Message::ToggleCalendarVisibility(cal.href.clone(), v));

                let mut label = button(text(&cal.name).size(16))
                    .width(Length::Fill)
                    .padding(10)
                    .on_press(Message::SelectCalendar(cal.href.clone()));

                label = if is_target {
                    label.style(button::primary)
                } else {
                    label.style(button::text)
                };

                let focus_btn = button(icon::icon(icon::ARROW_RIGHT).size(14))
                    .style(button::text)
                    .padding(10)
                    .on_press(Message::IsolateCalendar(cal.href.clone()));

                row![check, label, focus_btn] // Added focus_btn
                    .spacing(2)
                    .align_y(iced::Alignment::Center)
                    .into()
            })
            .collect::<Vec<_>>(),
    )
    .spacing(5)
    .width(Length::Fill);

    column![toggle_container, list].spacing(5).into()
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

    // --- HEADER ICONS (Unsynced + Refresh) ---
    let mut header_icons = row![].spacing(10).align_y(iced::Alignment::Center);

    if app.unsynced_changes {
        header_icons = header_icons.push(
            container(text("Unsynced").size(12).color(Color::WHITE))
                .style(|_| container::Style {
                    background: Some(Color::from_rgb(0.8, 0.5, 0.0).into()), // Orange
                    border: iced::Border {
                        radius: 4.0.into(),
                        ..Default::default()
                    },
                    ..Default::default()
                })
                .padding(5),
        );
    }

    header_icons = header_icons.push(
        button(icon::icon(icon::REFRESH).size(20)) // Refresh Icon (slightly larger)
            .style(button::secondary)
            .padding(6)
            .on_press(Message::Refresh),
    );

    let search_input = text_input("Search...", &app.search_value)
        .on_input(Message::SearchChanged)
        .padding(5)
        .size(16);

    // --- EXPORT LOGIC ---
    let mut export_ui: Element<'_, Message> = row![].into();

    if app.active_cal_href.as_deref() == Some(LOCAL_CALENDAR_HREF) {
        let targets: Vec<_> = app
            .calendars
            .iter()
            .filter(|c| c.href != LOCAL_CALENDAR_HREF && !app.disabled_calendars.contains(&c.href))
            .collect();

        if !targets.is_empty() {
            let mut row = row![
                text("Export to:")
                    .size(14)
                    .color(Color::from_rgb(0.5, 0.5, 0.5))
            ]
            .spacing(5)
            .align_y(iced::Alignment::Center);

            for cal in targets {
                row = row.push(
                    button(text(&cal.name).size(12))
                        .style(button::secondary)
                        .padding(5)
                        .on_press(Message::MigrateLocalTo(cal.href.clone())),
                );
            }
            export_ui = row.into();
        }
    }

    // --- ASSEMBLE HEADER ---
    let header = row![
        column![
            row![
                text(title_text).size(40),
                header_icons // Icons next to title
            ]
            .spacing(15)
            .align_y(iced::Alignment::Center),
            export_ui
        ]
        .spacing(5),
        horizontal_space(),
        search_input.width(200)
    ]
    .align_y(iced::Alignment::Center);

    let input_area = view_input_area(app);

    // --- MAIN COLUMN ASSEMBLY ---
    let mut main_col = column![header, input_area];

    // --- ERROR / OFFLINE BANNER ---
    if let Some(err) = &app.error_msg {
        // Create a row with the text and a close button
        let error_content = row![
            text(err).color(Color::WHITE).size(14).width(Length::Fill),
            button(icon::icon(icon::CROSS).size(14).color(Color::WHITE))
                .style(button::text) // Transparent button style
                .padding(2)
                .on_press(Message::DismissError)
        ]
        .align_y(iced::Alignment::Center);

        main_col = main_col.push(
            container(error_content)
                .width(Length::Fill)
                .padding(5)
                .style(|_| container::Style {
                    background: Some(Color::from_rgb(0.8, 0.2, 0.2).into()),
                    ..Default::default()
                }),
        );
    }

    let tasks_view = column(
        app.tasks
            .iter()
            .enumerate()
            .map(|(real_index, task)| view_task_row(app, real_index, task))
            .collect::<Vec<_>>(),
    )
    .spacing(1);

    main_col = main_col.push(scrollable(tasks_view).height(Length::Fill));

    container(main_col.spacing(20).padding(20).max_width(800)).into()
}

fn view_input_area(app: &GuiApp) -> Element<'_, Message> {
    let input_placeholder = if app.editing_uid.is_some() {
        "Edit Title...".to_string()
    } else {
        // Show which calendar we are writing to
        let target_name = app
            .calendars
            .iter()
            .find(|c| Some(&c.href) == app.active_cal_href.as_ref())
            .map(|c| c.name.as_str())
            .unwrap_or("Default");

        format!(
            "Add task to {} (e.g. Buy cat food !1 @weekly #groceries ~30m)",
            target_name
        )
    };

    // 1. Main Text Input
    let input_title = text_input(&input_placeholder, &app.input_value)
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

        if let Some(edit_uid) = &app.editing_uid
            && let Some(task) = app.tasks.iter().find(|t| t.uid == *edit_uid)
        {
            // Filter: Exclude current calendar AND hidden calendars
            let targets: Vec<_> = app
                .calendars
                .iter()
                .filter(|c| {
                    c.href != task.calendar_href && !app.disabled_calendars.contains(&c.href)
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
    // Further reduce per-depth indent so checkboxes start closer to the left
    let indent_size = if show_indent { task.depth * 12 } else { 0 };
    let indent = horizontal_space().width(Length::Fixed(indent_size as f32));

    // 2. Title Row (Just Summary) - replaced later with direct text in-line to avoid wrapping issues

    // Build tags_element on demand so we can reuse the construction in
    // multiple layout branches without moving a value twice.
    let build_tags = || -> Element<'a, Message> {
        let mut tags_row: iced::widget::Row<'_, Message> = row![].spacing(3);

        if is_blocked {
            tags_row = tags_row.push(
                container(text("[Blocked]").size(12).color(Color::WHITE))
                    .style(|_| container::Style {
                        background: Some(Color::from_rgb(0.8, 0.2, 0.2).into()),
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

        if task.rrule.is_some() {
            tags_row = tags_row.push(container(icon::icon(icon::REPEAT).size(14)).padding(0));
        }

        tags_row.into()
    };

    // Reserve a modest width for the date so the title has more room; reduce wasted gap after date
    let date_text: Element<'a, Message> = match task.due {
        Some(d) => container(
            text(d.format("%Y-%m-%d").to_string())
                .size(14)
                .color(Color::from_rgb(0.5, 0.5, 0.5)),
        )
        .width(Length::Fixed(80.0))
        .into(),
        // If no date is set, don't reserve space so tags/actions can use it
        None => horizontal_space().width(Length::Fixed(0.0)).into(),
    };

    // 4. Info Button
    let has_desc = !task.description.is_empty();
    let has_deps = !task.dependencies.is_empty();
    let is_expanded = app.expanded_tasks.contains(&task.uid);

    // Reserve a fixed slot for the info button so the date doesn't shift when details toggle.
    // We'll insert either a real button (when content exists) or a transparent placeholder of the same width.

    // 5. Actions
    // Start empty; we'll push an info button only when there's something to show.
    // Slightly tighter spacing between action buttons
    let mut actions = row![].spacing(3);

    // Info button slot: real button when content exists, placeholder otherwise
    if has_desc || has_deps {
        let info_btn = button(icon::icon(icon::INFO).size(12))
            .style(if is_expanded {
                button::primary
            } else {
                button::secondary
            })
            .padding(4)
            .width(Length::Fixed(25.0))
            .on_press(Message::ToggleDetails(task.uid.clone()));
        actions = actions.push(info_btn);
    } else {
        // Invisible placeholder to hold layout space (same width as info button)
        actions = actions.push(horizontal_space().width(Length::Fixed(25.0)));
    }

    if let Some(yanked) = &app.yanked_uid {
        if *yanked != task.uid {
            actions = actions.push(
                button(text("Block").size(14))
                    .style(button::secondary)
                    .padding(4)
                    .on_press(Message::AddDependency(task.uid.clone())),
            );
            actions = actions.push(
                button(text("Child").size(14))
                    .style(button::secondary)
                    .padding(4)
                    .on_press(Message::MakeChild(task.uid.clone())),
            );
        } else {
            actions = actions.push(
                button(icon::icon(icon::UNLINK).size(14))
                    .style(button::primary)
                    .padding(4)
                    .on_press(Message::ClearYank),
            );
        }
    } else {
        actions = actions.push(
            button(icon::icon(icon::LINK).size(14))
                .style(button::secondary)
                .padding(4)
                .on_press(Message::YankTask(task.uid.clone())),
        );
    }

    let btn_style = button::secondary;

    // Status Controls (action button on the right)
    if task.status != crate::model::TaskStatus::Completed
        && task.status != crate::model::TaskStatus::Cancelled
    {
        // Use a codicon play (eb2c) for the action when starting, and PAUSE when stopping.
        let (action_icon, msg_status) = if task.status == crate::model::TaskStatus::InProcess {
            (icon::PAUSE, crate::model::TaskStatus::NeedsAction)
        } else {
            (icon::PLAY, crate::model::TaskStatus::InProcess)
        };
        actions = actions.push(
            button(icon::icon(action_icon).size(14))
                .style(btn_style)
                .padding(4)
                .on_press(Message::SetTaskStatus(index, msg_status)),
        );
    }

    actions = actions.push(
        button(icon::icon(icon::PLUS).size(14))
            .style(btn_style)
            .padding(4)
            .on_press(Message::ChangePriority(index, 1)),
    );
    actions = actions.push(
        button(icon::icon(icon::MINUS).size(14))
            .style(btn_style)
            .padding(4)
            .on_press(Message::ChangePriority(index, -1)),
    );
    actions = actions.push(
        button(icon::icon(icon::EDIT).size(14))
            .style(btn_style)
            .padding(4)
            .on_press(Message::EditTaskStart(index)),
    );

    // Cancel Button (Moved here)
    if task.status != crate::model::TaskStatus::Completed
        && task.status != crate::model::TaskStatus::Cancelled
    {
        actions = actions.push(
            button(icon::icon(icon::CROSS).size(14))
                .style(button::danger)
                .padding(4)
                .on_press(Message::SetTaskStatus(
                    index,
                    crate::model::TaskStatus::Cancelled,
                )),
        );
    }

    actions = actions.push(
        button(icon::icon(icon::TRASH).size(14))
            .style(button::danger)
            .padding(4)
            .on_press(Message::DeleteTask(index)),
    );

    // 6. Construct Main Row

    // --- CUSTOM MULTI-STATE CHECKBOX ---
    // We define the look (Icon + Background Color) based on status
    let (icon_char, bg_color, border_color) = match task.status {
        // Show a play icon for ongoing tasks per user's preference (nf-cod-play)
        crate::model::TaskStatus::InProcess => (
            // Use the FontAwesome play inside the status box (prettier in-box glyph)
            icon::PLAY_FA,
            // Toned down / Muted Green
            Color::from_rgb(0.6, 0.8, 0.6),
            Color::from_rgb(0.4, 0.5, 0.4),
        ),
        crate::model::TaskStatus::Cancelled => (
            icon::CROSS,
            Color::from_rgb(0.3, 0.2, 0.2),
            Color::from_rgb(0.5, 0.4, 0.4),
        ),
        crate::model::TaskStatus::Completed => (
            icon::CHECK,
            // The "Pretty" Bright Green
            Color::from_rgb(0.0, 0.6, 0.0),
            Color::from_rgb(0.0, 0.8, 0.0),
        ),
        // NeedsAction: render an empty interior (no glyph) so we avoid a box-within-box
        crate::model::TaskStatus::NeedsAction => {
            (' ', Color::TRANSPARENT, Color::from_rgb(0.5, 0.5, 0.5))
        }
    };

    // We use a Button fixed to 24x24 to act as a Checkbox
    let status_btn = button(
        container(
            // If icon_char is a space sentinel, render an empty Text so the colored box appears clean
            if icon_char != ' ' {
                icon::icon(icon_char).size(12).color(Color::WHITE)
            } else {
                text("").size(12).color(Color::WHITE)
            },
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
    // Build the title row (Fill) and a main text column (title + tags)
    // Decide whether to place tags inline with the title or below it.
    // We can't know pixel width here, so use a simple heuristic based on
    // the title length and number of tags. If the combined estimated
    // length is small, render tags inline; otherwise render tags on a
    // separate right-aligned row below the title.
    let title_chars = task.summary.chars().count();
    let est_tags_len = task.categories.len() * 4
        + if task.estimated_duration.is_some() {
            3
        } else {
            0
        }
        + if task.rrule.is_some() { 1 } else { 0 }
        + if is_blocked { 9 } else { 0 };
    // threshold tuned to typical widths; tweak as needed
    let place_inline = (title_chars + est_tags_len) <= 60;

    // Helper boolean to check if we have ANY metadata to display (Tags, Duration, Recurrence, Blocked)
    let has_metadata = !task.categories.is_empty()
        || task.rrule.is_some()
        || is_blocked
        || task.estimated_duration.is_some();

    let title_row = if place_inline {
        row![
            text(&task.summary)
                .size(20)
                .color(color)
                .width(Length::Fill),
            // show tags inline to the right when small enough
            if has_metadata {
                build_tags()
            } else {
                horizontal_space().width(Length::Fixed(0.0)).into()
            }
        ]
        .spacing(6)
        .align_y(iced::Alignment::Center)
    } else {
        row![
            text(&task.summary)
                .size(20)
                .color(color)
                .width(Length::Fill)
        ]
        .spacing(6)
        .align_y(iced::Alignment::Center)
    };

    let main_text_col = column![
        title_row,
        // If we didn't place tags inline, show them on a separate right-aligned row
        if !place_inline && has_metadata {
            row![horizontal_space(), build_tags()]
        } else {
            row![]
        }
    ]
    .width(Length::Fill)
    .spacing(1);

    // Place date_text as its own column element so it doesn't shift when actions change.
    // Tighten spacing so elements use space more efficiently
    let row_main = row![indent, status_btn, main_text_col, date_text, actions]
        .spacing(10)
        .align_y(iced::Alignment::Center);

    // Reduce padding so the row is more compact; nudge right edge 1px left to avoid scrollbar overlap
    let padded_row = container(row_main).padding(iced::Padding {
        top: 4.0,
        right: 12.0,
        bottom: 4.0,
        left: 0.0,
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
                button(icon::icon(icon::CROSS).size(10))
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
                    button(icon::icon(icon::CROSS).size(10))
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

            // Build a list of target calendars (excluding the current one and hidden calendars)
            let targets: Vec<_> = app
                .calendars
                .iter()
                .filter(|c| c.href != current_cal_href && !app.disabled_calendars.contains(&c.href))
                .collect();

            let move_label = text("Move to:")
                .size(12)
                .color(Color::from_rgb(0.5, 0.5, 0.5));

            // We use a Row of buttons for targets, but make it horizontally scrollable and constrained in height
            let mut move_row = row![].spacing(5).align_y(iced::Alignment::Center);

            for cal in targets {
                move_row = move_row.push(
                    button(text(&cal.name).size(10))
                        .style(button::secondary)
                        .padding(3)
                        .on_press(Message::MoveTask(task.uid.clone(), cal.href.clone())),
                );
            }

            details_col = details_col.push(
                row![move_label, scrollable(move_row).height(Length::Fixed(30.0))]
                    .spacing(10)
                    .align_y(iced::Alignment::Center),
            );
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
                button(icon::icon(icon::CROSS).size(12))
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
            // Logic inverted: Checkbox checked = Enabled (!Disabled)
            let is_enabled = !app.disabled_calendars.contains(&cal.href);

            let row_content = row![
                checkbox(&cal.name, is_enabled)
                    // When toggled, we send !v because the msg is "ToggleDisabled"
                    .on_toggle(move |v| Message::ToggleCalendarDisabled(cal.href.clone(), !v))
                    .width(Length::Fill)
            ];

            col = col.push(row_content.spacing(10).align_y(iced::Alignment::Center));
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

    // Initialize the buttons row before using it
    let mut buttons = row![].spacing(10);

    if !is_settings {
        // Onboarding screen
        buttons = buttons.push(
            button("Use Offline Mode")
                .padding(10)
                .style(button::secondary)
                .on_press(Message::ObSubmitOffline),
        );
    }

    if is_settings {
        // Settings screen
        buttons = buttons.push(
            button("Cancel")
                .padding(10)
                .style(button::secondary)
                .on_press(Message::CancelSettings),
        );
    }

    // This button appears on both screens
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
    .max_width(500);

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
