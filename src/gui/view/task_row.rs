// File: src/gui/view/task_row.rs

use crate::color_utils;
use crate::gui::icon;
use crate::gui::message::Message;
use crate::gui::state::GuiApp;
use crate::model::Task as TodoTask;
use std::collections::HashSet;
use std::time::Duration;

use super::tooltip_style;
use iced::widget::{Space, button, column, container, row, scrollable, text, tooltip};
pub use iced::widget::{rich_text, span};
use iced::{Border, Color, Element, Length, Theme};

pub fn view_task_row<'a>(
    app: &'a GuiApp,
    index: usize,
    task: &'a TodoTask,
) -> Element<'a, Message> {
    let is_blocked = app.store.is_blocked(task);
    let is_selected = app.selected_uid.as_ref() == Some(&task.uid);
    let color = if is_blocked {
        Color::from_rgb(0.5, 0.5, 0.5)
    } else {
        // Priority Gradient: Red (Hot) -> Yellow (Normal) -> Purple/Slate (Cold)
        match task.priority {
            // 1: Red
            1 => Color::from_rgb(1.0, 0.2, 0.2),
            // 2: Orange-Red
            2 => Color::from_rgb(1.0, 0.4, 0.2),
            // 3: Orange
            3 => Color::from_rgb(1.0, 0.6, 0.2),
            // 4: Amber
            4 => Color::from_rgb(1.0, 0.8, 0.2),
            // 5: Yellow
            5 => Color::from_rgb(1.0, 1.0, 0.2),
            // 6: Pale Khaki (Desaturating)
            6 => Color::from_rgb(0.85, 0.85, 0.55),
            // 7: Light Steel Blue (Cooling)
            7 => Color::from_rgb(0.7, 0.75, 0.85),
            // 8: Slate / Muted Purple
            8 => Color::from_rgb(0.65, 0.6, 0.8),
            // 9: Greyish Lavender
            9 => Color::from_rgb(0.6, 0.55, 0.65),
            // 0: White
            _ => Color::WHITE,
        }
    };
    let show_indent = app.active_cal_href.is_some() && app.search_value.is_empty();
    let indent_size = if show_indent { task.depth * 12 } else { 0 };
    let indent = Space::new().width(Length::Fixed(indent_size as f32));

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

    // --- Build Tags Logic ---
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

        // --- FIXED: Consolidated hiding logic ---
        // 1. Calculate tags to hide because they are inherited from the parent task.
        let mut tags_to_hide: HashSet<String> = if show_indent && let Some(p_uid) = &task.parent_uid
        {
            let mut p_cats = HashSet::new();
            if let Some(href) = app.store.index.get(p_uid)
                && let Some(list) = app.store.calendars.get(href)
                && let Some(p) = list.iter().find(|t| t.uid == *p_uid)
            {
                p_cats = p.categories.iter().cloned().collect();
            }
            p_cats
        } else {
            HashSet::new()
        };

        // 2. Add tags to hide because they are implied by an alias.
        for cat in &task.categories {
            let mut search = cat.as_str();
            loop {
                if let Some(targets) = app.tag_aliases.get(search) {
                    for t in targets {
                        tags_to_hide.insert(t.clone());
                    }
                }
                if let Some(idx) = search.rfind(':') {
                    search = &search[..idx];
                } else {
                    break;
                }
            }
        }

        for cat in &task.categories {
            // Hide if parent has same tag OR if it is an expanded alias
            if tags_to_hide.contains(cat) {
                continue;
            }
            // --- END FIX ---

            let (r, g, b) = color_utils::generate_color(cat);
            let bg_color = Color::from_rgb(r, g, b);
            let text_color = if color_utils::is_dark(r, g, b) {
                Color::WHITE
            } else {
                Color::BLACK
            };
            tags_row = tags_row.push(
                button(text(format!("#{}", cat)).size(12).color(text_color))
                    .style(move |_theme, status| {
                        let base = button::Style {
                            background: Some(bg_color.into()),
                            text_color,
                            border: iced::Border {
                                radius: 4.0.into(),
                                ..Default::default()
                            },
                            ..button::Style::default()
                        };
                        match status {
                            button::Status::Hovered | button::Status::Pressed => button::Style {
                                border: iced::Border {
                                    color: Color::BLACK.scale_alpha(0.2),
                                    width: 1.0,
                                    radius: 4.0.into(),
                                },
                                ..base
                            },
                            _ => base,
                        }
                    })
                    .padding(3)
                    .on_press(Message::JumpToTag(cat.clone())),
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

    let date_text: Element<'a, Message> = match task.due {
        Some(d) => container(
            text(d.format("%Y-%m-%d").to_string())
                .size(14)
                .color(Color::from_rgb(0.5, 0.5, 0.5)),
        )
        .width(Length::Fixed(80.0))
        .into(),
        None => Space::new().width(Length::Fixed(0.0)).into(),
    };

    let has_desc = !task.description.is_empty();
    let has_deps = !task.dependencies.is_empty();
    let is_expanded = app.expanded_tasks.contains(&task.uid);

    let mut actions = row![].spacing(3);

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
        // Apply tooltip_style
        actions = actions.push(
            tooltip(
                info_btn,
                text("Show details").size(12),
                tooltip::Position::Top,
            )
            .style(tooltip_style)
            .delay(Duration::from_millis(700)),
        );
    } else {
        actions = actions.push(Space::new().width(Length::Fixed(25.0)));
    }

    if let Some(yanked) = &app.yanked_uid {
        if *yanked != task.uid {
            let block_btn = button(icon::icon(icon::BLOCKED).size(14))
                .style(action_style)
                .padding(4)
                .on_press(Message::AddDependency(task.uid.clone()));
            actions = actions.push(
                tooltip(
                    block_btn,
                    text("Block (depends on)").size(12),
                    tooltip::Position::Top,
                )
                .style(tooltip_style)
                .delay(Duration::from_millis(700)),
            );
            let child_btn = button(icon::icon(icon::CHILD).size(14))
                .style(action_style)
                .padding(4)
                .on_press(Message::MakeChild(task.uid.clone()));
            actions = actions.push(
                tooltip(
                    child_btn,
                    text("Make child").size(12),
                    tooltip::Position::Top,
                )
                .style(tooltip_style)
                .delay(Duration::from_millis(700)),
            );
        } else {
            let unlink_btn = button(icon::icon(icon::UNLINK).size(14))
                .style(button::primary)
                .padding(4)
                .on_press(Message::ClearYank);
            actions = actions.push(
                tooltip(unlink_btn, text("Unlink").size(12), tooltip::Position::Top)
                    .style(tooltip_style)
                    .delay(Duration::from_millis(700)),
            );
            let create_child_btn = button(icon::icon(icon::CREATE_CHILD).size(14))
                .style(button::primary)
                .padding(4)
                .on_press(Message::StartCreateChild(task.uid.clone()));
            actions = actions.push(
                tooltip(
                    create_child_btn,
                    text("Create subtask").size(12),
                    tooltip::Position::Top,
                )
                .style(tooltip_style)
                .delay(Duration::from_millis(700)),
            );

            // ELEVATOR UP (Moved here as requested)
            if task.parent_uid.is_some() {
                let lift_btn = button(icon::icon(icon::ELEVATOR_UP).size(14))
                    .style(action_style)
                    .padding(4)
                    .on_press(Message::RemoveParent(task.uid.clone()));
                actions = actions.push(
                    tooltip(
                        lift_btn,
                        text("Promote (remove parent)").size(12),
                        tooltip::Position::Top,
                    )
                    .style(tooltip_style)
                    .delay(Duration::from_millis(700)),
                );
            }
        }
    } else {
        let link_btn = button(icon::icon(icon::LINK).size(14))
            .style(action_style)
            .padding(4)
            .on_press(Message::YankTask(task.uid.clone()));
        actions = actions.push(
            tooltip(
                link_btn,
                text("Yank (copy ID)").size(12),
                tooltip::Position::Top,
            )
            .style(tooltip_style)
            .delay(Duration::from_millis(700)),
        );
    }

    if task.status != crate::model::TaskStatus::Completed
        && task.status != crate::model::TaskStatus::Cancelled
    {
        let (action_icon, msg_status, tooltip_text) =
            if task.status == crate::model::TaskStatus::InProcess {
                (
                    icon::PAUSE,
                    crate::model::TaskStatus::NeedsAction,
                    "Pause Task",
                )
            } else {
                (
                    icon::PLAY,
                    crate::model::TaskStatus::InProcess,
                    "Start Task",
                )
            };
        let status_toggle_btn = button(icon::icon(action_icon).size(14))
            .style(action_style)
            .padding(4)
            .on_press(Message::SetTaskStatus(index, msg_status));
        actions = actions.push(
            tooltip(
                status_toggle_btn,
                text(tooltip_text).size(12),
                tooltip::Position::Top,
            )
            .style(tooltip_style)
            .delay(Duration::from_millis(700)),
        );
    }

    let plus_btn = button(icon::icon(icon::PLUS).size(14))
        .style(action_style)
        .padding(4)
        .on_press(Message::ChangePriority(index, 1));
    actions = actions.push(
        tooltip(
            plus_btn,
            text("Increase priority").size(12),
            tooltip::Position::Top,
        )
        .style(tooltip_style)
        .delay(Duration::from_millis(700)),
    );

    let minus_btn = button(icon::icon(icon::MINUS).size(14))
        .style(action_style)
        .padding(4)
        .on_press(Message::ChangePriority(index, -1));
    actions = actions.push(
        tooltip(
            minus_btn,
            text("Decrease priority").size(12),
            tooltip::Position::Top,
        )
        .style(tooltip_style)
        .delay(Duration::from_millis(700)),
    );

    let edit_btn = button(icon::icon(icon::EDIT).size(14))
        .style(action_style)
        .padding(4)
        .on_press(Message::EditTaskStart(index));
    actions = actions.push(
        tooltip(edit_btn, text("Edit").size(12), tooltip::Position::Top)
            .style(tooltip_style)
            .delay(Duration::from_millis(700)),
    );

    let delete_btn = button(icon::icon(icon::TRASH).size(14))
        .style(danger_style)
        .padding(4)
        .on_press(Message::DeleteTask(index));
    actions = actions.push(
        tooltip(delete_btn, text("Delete").size(12), tooltip::Position::Top)
            .style(tooltip_style)
            .delay(Duration::from_millis(700)),
    );

    if task.status != crate::model::TaskStatus::Completed
        && task.status != crate::model::TaskStatus::Cancelled
    {
        let cancel_btn = button(icon::icon(icon::CROSS).size(14))
            .style(danger_style)
            .padding(4)
            .on_press(Message::SetTaskStatus(
                index,
                crate::model::TaskStatus::Cancelled,
            ));
        actions = actions.push(
            tooltip(cancel_btn, text("Cancel").size(12), tooltip::Position::Top)
                .style(tooltip_style)
                .delay(Duration::from_millis(700)),
        );
    }

    let (icon_char, bg_color, border_color) = match task.status {
        crate::model::TaskStatus::InProcess => (
            icon::PLAY_FA,
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
            Color::from_rgb(0.0, 0.6, 0.0),
            Color::from_rgb(0.0, 0.8, 0.0),
        ),
        crate::model::TaskStatus::NeedsAction => {
            (' ', Color::TRANSPARENT, Color::from_rgb(0.5, 0.5, 0.5))
        }
    };
    let status_btn = button(
        container(if icon_char != ' ' {
            icon::icon(icon_char).size(12).color(Color::WHITE)
        } else {
            text("").size(12).color(Color::WHITE)
        })
        .width(Length::Fill)
        .height(Length::Fill)
        .align_x(iced::alignment::Horizontal::Center)
        .align_y(iced::alignment::Vertical::Center),
    )
    .width(Length::Fixed(24.0))
    .height(Length::Fixed(24.0))
    .padding(0)
    .on_press(Message::ToggleTask(index, true))
    .style(move |_theme, status| {
        let base_active = button::Style {
            background: Some(bg_color.into()),
            text_color: Color::WHITE,
            border: iced::Border {
                color: border_color,
                width: 1.0,
                radius: 4.0.into(),
            },
            ..button::Style::default()
        };
        match status {
            iced::widget::button::Status::Hovered => button::Style {
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

    let title_chars = task.summary.chars().count();
    let est_tags_len = task.categories.len() * 4
        + if task.estimated_duration.is_some() {
            3
        } else {
            0
        }
        + if task.rrule.is_some() { 1 } else { 0 }
        + if is_blocked { 9 } else { 0 };
    let place_inline = (title_chars + est_tags_len) <= 60;
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
        if !place_inline && has_metadata {
            row![Space::new().width(Length::Fill), build_tags()]
        } else {
            row![]
        }
    ]
    .width(Length::Fill)
    .spacing(1);
    let row_main = row![indent, status_btn, main_text_col, date_text, actions]
        .spacing(10)
        .align_y(iced::Alignment::Center);

    let mut padded_row = container(row_main).padding(iced::Padding {
        top: 2.0,
        right: 16.0,
        bottom: 2.0,
        left: 6.0,
    });
    if is_selected {
        padded_row = padded_row.style(|theme: &Theme| {
            let palette = theme.extended_palette();
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

    let row_id = iced::widget::Id::from(task.uid.clone());

    if is_expanded {
        let mut details_col = column![].spacing(5);
        if !task.description.is_empty() {
            details_col = details_col.push(
                text(&task.description)
                    .size(14)
                    .color(Color::from_rgb(0.7, 0.7, 0.7)),
            );
        }
        if let Some(p_uid) = &task.parent_uid {
            let p_name = app
                .store
                .get_summary(p_uid)
                .unwrap_or_else(|| "Unknown Parent".to_string());
            let remove_parent_btn = button(icon::icon(icon::CROSS).size(10))
                .style(button::danger)
                .padding(2)
                .on_press(Message::RemoveParent(task.uid.clone()));
            // Apply tooltip_style
            let row = row![
                text("Parent:")
                    .size(12)
                    .color(Color::from_rgb(0.4, 0.8, 0.4)),
                text(p_name).size(12),
                tooltip(
                    remove_parent_btn,
                    text("Remove parent").size(12),
                    tooltip::Position::Top
                )
                .style(tooltip_style)
                .delay(Duration::from_millis(700))
            ]
            .spacing(5)
            .align_y(iced::Alignment::Center);
            details_col = details_col.push(row);
        }
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
                let remove_dep_btn = button(icon::icon(icon::CROSS).size(10))
                    .style(button::danger)
                    .padding(2)
                    .on_press(Message::RemoveDependency(task.uid.clone(), dep_uid.clone()));
                // Apply tooltip_style
                let dep_row = row![
                    text(format!("{} {}", check, name))
                        .size(12)
                        .color(Color::from_rgb(0.6, 0.6, 0.6)),
                    tooltip(
                        remove_dep_btn,
                        text("Remove dependency").size(12),
                        tooltip::Position::Top
                    )
                    .style(tooltip_style)
                    .delay(Duration::from_millis(700))
                ]
                .spacing(5)
                .align_y(iced::Alignment::Center);
                details_col = details_col.push(dep_row);
            }
        }
        if app.calendars.len() > 1 {
            let current_cal_href = task.calendar_href.clone();
            let targets: Vec<_> = app
                .calendars
                .iter()
                .filter(|c| c.href != current_cal_href && !app.disabled_calendars.contains(&c.href))
                .collect();
            let move_label = text("Move to:")
                .size(12)
                .color(Color::from_rgb(0.5, 0.5, 0.5));
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
