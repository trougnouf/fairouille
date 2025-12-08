// File: src/gui/view/task_row.rs
use crate::gui::icon;
use crate::gui::message::Message;
use crate::gui::state::GuiApp;
use crate::model::Task as TodoTask;

use iced::widget::{Space, button, column, container, row, scrollable, text};
use iced::{Border, Color, Element, Length, Theme};

pub fn view_task_row<'a>(
    app: &'a GuiApp,
    index: usize,
    task: &'a TodoTask,
) -> Element<'a, Message> {
    // 1. Check Blocked Status
    let is_blocked = app.store.is_blocked(task);

    // Check if selected
    let is_selected = app.selected_uid.as_ref() == Some(&task.uid);

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
    let indent = Space::new().width(Length::Fixed(indent_size as f32));

    // --- CUSTOM STYLES ---

    // Style for standard actions (Edit, Move, Priority, etc.)
    // Idle: Transparent with light text
    // Hover: Dark background with white text
    let action_style = |theme: &Theme, status: button::Status| -> button::Style {
        let palette = theme.extended_palette();
        let base = button::Style {
            background: Some(Color::TRANSPARENT.into()),
            text_color: palette.background.weak.text,
            border: Border::default(),
            ..button::Style::default()
        };

        match status {
            button::Status::Active => base,
            button::Status::Hovered => button::Style {
                background: Some(palette.background.weak.color.into()),
                text_color: Color::WHITE,
                border: Border {
                    radius: 4.0.into(),
                    ..Border::default()
                },
                ..base
            },
            button::Status::Pressed => button::Style {
                background: Some(palette.background.strong.color.into()),
                text_color: Color::WHITE,
                border: Border {
                    radius: 4.0.into(),
                    ..Border::default()
                },
                ..base
            },
            button::Status::Disabled => button::Style {
                text_color: palette.background.weak.text.scale_alpha(0.3),
                ..base
            },
        }
    };

    // Style for destructive actions (Delete, Cancel)
    // Idle: Transparent with Red Icon
    // Hover: Red Background with White Icon
    let danger_style = |theme: &Theme, status: button::Status| -> button::Style {
        let palette = theme.extended_palette();
        let base = button::Style {
            background: Some(Color::TRANSPARENT.into()),
            text_color: palette.danger.base.color,
            border: Border::default(),
            ..button::Style::default()
        };

        match status {
            button::Status::Active => base,
            button::Status::Hovered => button::Style {
                background: Some(palette.danger.base.color.into()),
                text_color: Color::WHITE,
                border: Border {
                    radius: 4.0.into(),
                    ..Border::default()
                },
                ..base
            },
            button::Status::Pressed => button::Style {
                background: Some(palette.danger.strong.color.into()),
                text_color: Color::WHITE,
                border: Border {
                    radius: 4.0.into(),
                    ..Border::default()
                },
                ..base
            },
            button::Status::Disabled => button::Style {
                text_color: palette.danger.base.color.scale_alpha(0.3),
                ..base
            },
        }
    };

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
        None => Space::new().width(Length::Fixed(0.0)).into(),
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
                action_style
            })
            .padding(4)
            .width(Length::Fixed(25.0))
            .on_press(Message::ToggleDetails(task.uid.clone()));
        actions = actions.push(info_btn);
    } else {
        // Invisible placeholder to hold layout space (same width as info button)
        actions = actions.push(Space::new().width(Length::Fixed(25.0)));
    }

    if let Some(yanked) = &app.yanked_uid {
        if *yanked != task.uid {
            actions = actions.push(
                button(icon::icon(icon::BLOCKED).size(14))
                    .style(action_style)
                    .padding(4)
                    .on_press(Message::AddDependency(task.uid.clone())),
            );
            actions = actions.push(
                button(icon::icon(icon::CHILD).size(14))
                    .style(action_style)
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
            actions = actions.push(
                button(icon::icon(icon::CREATE_CHILD).size(14))
                    .style(button::primary)
                    .padding(4)
                    .on_press(Message::StartCreateChild(task.uid.clone())),
            );
        }
    } else {
        actions = actions.push(
            button(icon::icon(icon::LINK).size(14))
                .style(action_style)
                .padding(4)
                .on_press(Message::YankTask(task.uid.clone())),
        );
    }

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
                .style(action_style)
                .padding(4)
                .on_press(Message::SetTaskStatus(index, msg_status)),
        );
    }

    actions = actions.push(
        button(icon::icon(icon::PLUS).size(14))
            .style(action_style)
            .padding(4)
            .on_press(Message::ChangePriority(index, 1)),
    );
    actions = actions.push(
        button(icon::icon(icon::MINUS).size(14))
            .style(action_style)
            .padding(4)
            .on_press(Message::ChangePriority(index, -1)),
    );
    actions = actions.push(
        button(icon::icon(icon::EDIT).size(14))
            .style(action_style)
            .padding(4)
            .on_press(Message::EditTaskStart(index)),
    );

    // --- REORDERED HERE: Delete first, then Cancel ---

    // 1. Delete Button (Dangerously close prevention: Put it "inside")
    actions = actions.push(
        button(icon::icon(icon::TRASH).size(14))
            .style(danger_style)
            .padding(4)
            .on_press(Message::DeleteTask(index)),
    );

    // 2. Cancel Button (Safer on the edge)
    if task.status != crate::model::TaskStatus::Completed
        && task.status != crate::model::TaskStatus::Cancelled
    {
        actions = actions.push(
            button(icon::icon(icon::CROSS).size(14))
                .style(danger_style)
                .padding(4)
                .on_press(Message::SetTaskStatus(
                    index,
                    crate::model::TaskStatus::Cancelled,
                )),
        );
    }

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
                Space::new().width(Length::Fixed(0.0)).into()
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
            row![Space::new().width(Length::Fill), build_tags()]
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

    // --- CHANGED HERE: Increased right padding from 6.0 to 16.0 ---
    let mut padded_row = container(row_main).padding(iced::Padding {
        top: 2.0,
        right: 16.0,
        bottom: 2.0,
        left: 6.0,
    });

    if is_selected {
        padded_row = padded_row.style(|theme: &Theme| {
            let palette = theme.extended_palette();
            // Use warning color (typically yellow/orange) for a "happy" highlight
            container::Style {
                background: Some(
                    Color {
                        a: 0.05,
                        ..palette.warning.base.color
                    }
                    .into(),
                ),
                border: iced::Border {
                    color: Color {
                        a: 0.5,
                        ..palette.warning.base.color
                    },
                    width: 1.0,
                    radius: 4.0.into(),
                },
                ..Default::default()
            }
        });
    }

    // GENERATE ID
    let row_id = iced::widget::Id::from(task.uid.clone());

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
            Space::new().width(Length::Fixed(indent_size as f32 + 30.0)),
            details_col
        ];
        container(column![padded_row, desc_row].spacing(5))
            .padding(5)
            .id(row_id)
            .into()
    } else {
        padded_row.id(row_id).into()
    }
}
