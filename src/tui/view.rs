// File: src/tui/view.rs
use crate::color_utils;
use crate::store::UNCATEGORIZED_ID;
use crate::tui::action::SidebarMode;
use crate::tui::state::{AppState, Focus, InputMode};

use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph, Wrap},
};

pub fn draw(f: &mut Frame, state: &mut AppState) {
    let full_help_text = vec![
        Line::from(vec![
            Span::styled(
                " GLOBAL ",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(" Tab:Switch Focus  ?:Toggle Help  q:Quit"),
        ]),
        Line::from(vec![
            Span::styled(
                " NAVIGATION ",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(" j/k:Up/Down  PgUp/PgDn:Scroll"),
        ]),
        Line::from(vec![
            Span::styled(
                " TASKS ",
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(" a:Add  e:Edit Title  E:Edit Desc  d:Delete  Space:Toggle Done"),
        ]),
        Line::from(vec![
            Span::styled("       ", Style::default()), // Indent alignment
            Span::raw("s:Start/Pause  x:Cancel  M:Move  r:Sync  X:Export(Local)"),
        ]),
        Line::from(vec![
            Span::styled(
                " ORGANIZATION ",
                Style::default()
                    .fg(Color::Magenta)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(
                " +/-:Priority  </>:Indent  y:Yank  b:Block(w/Yank)  c:Child(w/Yank)  C:NewChild",
            ),
        ]),
        Line::from(vec![
            Span::styled(
                " VIEW & FILTER ",
                Style::default()
                    .fg(Color::Blue)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(" /:Search  H:Hide Completed  1:Cal View  2:Tag View"),
        ]),
        Line::from(vec![
            Span::styled(
                " SIDEBAR ",
                Style::default()
                    .fg(Color::LightCyan)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(
                " Enter:Select/Toggle  Space:Toggle Visibility  *:Show/Clear All  Right:Focus(Solo)",
            ),
        ]),
    ];

    let footer_height = if state.mode == InputMode::EditingDescription {
        Constraint::Length(10)
    } else if state.show_full_help {
        Constraint::Length(full_help_text.len() as u16 + 2)
    } else {
        Constraint::Length(3)
    };

    let v_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0), footer_height])
        .split(f.area());

    let h_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(25), Constraint::Percentage(75)])
        .split(v_chunks[0]);

    let main_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(70), Constraint::Percentage(30)])
        .split(h_chunks[1]);

    // --- Sidebar ---
    let sidebar_style = if state.active_focus == Focus::Sidebar {
        Style::default().fg(Color::Yellow)
    } else {
        Style::default()
    };

    let (sidebar_title, sidebar_items) = match state.sidebar_mode {
        SidebarMode::Calendars => {
            let items: Vec<ListItem> = state
                .calendars
                .iter()
                .filter(|c| !state.disabled_calendars.contains(&c.href))
                .map(|c| {
                    let is_target = Some(&c.href) == state.active_cal_href.as_ref();
                    let is_visible = !state.hidden_calendars.contains(&c.href);
                    let prefix = if is_target { ">" } else { " " };
                    let check = if is_visible { "[x]" } else { "[ ]" };
                    let color = if is_target {
                        Color::Yellow
                    } else {
                        Color::Reset
                    };
                    let style = if is_target {
                        Style::default().fg(color).add_modifier(Modifier::BOLD)
                    } else if !is_visible {
                        Style::default().fg(Color::DarkGray)
                    } else {
                        Style::default().fg(color)
                    };
                    ListItem::new(Line::from(format!("{} {} {}", prefix, check, c.name)))
                        .style(style)
                })
                .collect();
            (" Calendars [1] ".to_string(), items)
        }
        SidebarMode::Categories => {
            let all_cats = state.store.get_all_categories(
                state.hide_completed,
                state.hide_fully_completed_tags,
                &state.selected_categories,
                &state.hidden_calendars,
            );
            let items: Vec<ListItem> = all_cats
                .iter()
                .map(|(c, count)| {
                    let selected = if state.selected_categories.contains(c) {
                        "[x]"
                    } else {
                        "[ ]"
                    };
                    if c == UNCATEGORIZED_ID {
                        ListItem::new(Line::from(format!(
                            "{} Uncategorized ({})",
                            selected, count
                        )))
                    } else {
                        let (r, g, b) = color_utils::generate_color(c);
                        let color =
                            Color::Rgb((r * 255.0) as u8, (g * 255.0) as u8, (b * 255.0) as u8);
                        let spans = vec![
                            Span::raw(format!("{} ", selected)),
                            Span::styled("#", Style::default().fg(color)),
                            Span::raw(format!("{} ({})", c, count)),
                        ];
                        ListItem::new(Line::from(spans))
                    }
                })
                .collect();
            let logic = if state.match_all_categories {
                "AND"
            } else {
                "OR"
            };
            (format!(" Tags [2] ({}) ", logic), items)
        }
    };

    let sidebar = List::new(sidebar_items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(sidebar_title)
                .border_style(sidebar_style),
        )
        .highlight_style(
            Style::default()
                .add_modifier(Modifier::BOLD)
                .bg(Color::Blue),
        );
    f.render_stateful_widget(sidebar, h_chunks[0], &mut state.cal_state);

    // --- Task List ---
    // --- Task List ---
    let list_inner_width = main_chunks[0].width.saturating_sub(2) as usize;

    let task_items: Vec<ListItem> = state
        .tasks
        .iter()
        .map(|t| {
            let is_blocked = state.store.is_blocked(t);
            let base_style = if is_blocked {
                Style::default().fg(Color::DarkGray)
            } else {
                // Priority Gradient: Red (Hot) -> Yellow (Normal) -> Purple/Slate (Cold)
                match t.priority {
                    // 1: Critical -> Red
                    1 => Style::default().fg(Color::Red),
                    // 2: Urgent -> Orange-Red
                    2 => Style::default().fg(Color::Rgb(255, 69, 0)),
                    // 3: High -> Dark Orange
                    3 => Style::default().fg(Color::Rgb(255, 140, 0)),
                    // 4: Med-High -> Amber/Gold
                    4 => Style::default().fg(Color::Rgb(255, 190, 0)),
                    // 5: Normal -> Yellow
                    5 => Style::default().fg(Color::Yellow),
                    // 6: Med-Low -> Pale Goldenrod / Khaki (Desaturating)
                    6 => Style::default().fg(Color::Rgb(240, 230, 140)),
                    // 7: Low -> Light Steel Blue (Cooling down)
                    7 => Style::default().fg(Color::Rgb(176, 196, 222)),
                    // 8: Very Low -> Medium Purple / Slate
                    8 => Style::default().fg(Color::Rgb(147, 112, 219)),
                    // 9: Lowest -> Muted Lavender / Grey-Purple
                    9 => Style::default().fg(Color::Rgb(170, 150, 180)),
                    // 0: Unset -> White
                    _ => Style::default(),
                }
            };

            let checkbox = t.checkbox_symbol();
            let due_str = t
                .due
                .map(|d| format!(" ({})", d.format("%d/%m")))
                .unwrap_or_default();
            let dur_str = t.format_duration_short();
            let show_indent = state.active_cal_href.is_some() && state.mode != InputMode::Searching;
            let indent = if show_indent {
                "  ".repeat(t.depth)
            } else {
                "".to_string()
            };
            let recur_str = if t.rrule.is_some() { " (R)" } else { "" };

            // --- NEW: Calculate tags to hide based on aliases ---
            let mut hidden_tags = std::collections::HashSet::new();
            for cat in &t.categories {
                let mut search = cat.as_str();
                loop {
                    if let Some(targets) = state.tag_aliases.get(search) {
                        for target in targets {
                            hidden_tags.insert(target.clone());
                        }
                    }
                    if let Some(idx) = search.rfind(':') {
                        search = &search[..idx];
                    } else {
                        break;
                    }
                }
            }
            let visible_cats: Vec<&String> = t
                .categories
                .iter()
                .filter(|c| !hidden_tags.contains(*c))
                .collect();
            // --- END NEW ---

            // Layout Calculation (uses visible_cats now)
            let tags_str_len: usize = visible_cats.iter().map(|c| c.len() + 2).sum();

            let left_text = format!(
                "{}{}{} {}{}{}{}",
                indent,
                checkbox,
                if is_blocked { "[B] " } else { " " },
                t.summary,
                dur_str,
                due_str,
                recur_str
            );
            let total_len = left_text.chars().count() + tags_str_len;
            let padding_len = list_inner_width.saturating_sub(total_len);
            let padding = " ".repeat(padding_len);

            let mut spans = vec![Span::styled(left_text, base_style), Span::raw(padding)];

            // Use visible_cats for rendering
            for cat in visible_cats {
                let (r, g, b) = color_utils::generate_color(cat);
                let color = Color::Rgb((r * 255.0) as u8, (g * 255.0) as u8, (b * 255.0) as u8);
                spans.push(Span::styled(
                    format!(" #{}", cat),
                    Style::default().fg(color),
                ));
            }
            ListItem::new(Line::from(spans))
        })
        .collect();

    let mut title = if state.loading {
        " Tasks (Loading...) ".to_string()
    } else {
        format!(" Tasks ({}) ", state.tasks.len())
    };
    if state.unsynced_changes {
        title.push_str(" [UNSYNCED] ");
    }

    let main_style = if state.active_focus == Focus::Main {
        Style::default().fg(Color::Yellow)
    } else if state.unsynced_changes {
        Style::default().fg(Color::LightRed)
    } else {
        Style::default()
    };

    let task_list = List::new(task_items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(title)
                .border_style(main_style),
        )
        .highlight_style(
            Style::default()
                .add_modifier(Modifier::BOLD)
                .bg(Color::Green)
                .fg(Color::Black),
        );
    f.render_stateful_widget(task_list, main_chunks[0], &mut state.list_state);

    // Details
    let mut full_details = String::new();
    if let Some(task) = state.get_selected_task() {
        if !task.description.is_empty() {
            full_details.push_str(&task.description);
            full_details.push_str("\n\n");
        }
        if !task.dependencies.is_empty() {
            full_details.push_str("[Blocked By]:\n");
            for dep_uid in &task.dependencies {
                let name = state
                    .store
                    .get_summary(dep_uid)
                    .unwrap_or_else(|| "Unknown Task".to_string());
                let is_done = state.store.get_task_status(dep_uid).unwrap_or(false);
                let check = if is_done { "[x]" } else { "[ ]" };
                full_details.push_str(&format!(" {} {}\n", check, name));
            }
        }
    }
    if full_details.is_empty() {
        full_details = "No details.".to_string();
    }

    let details = Paragraph::new(full_details)
        .wrap(Wrap { trim: true })
        .block(Block::default().borders(Borders::ALL).title(" Details "));
    f.render_widget(details, main_chunks[1]);

    // Footer
    let footer_area = v_chunks[1];
    f.render_widget(Clear, footer_area);

    match state.mode {
        InputMode::Creating
        | InputMode::Editing
        | InputMode::Searching
        | InputMode::EditingDescription => {
            // ... Input Mode Rendering logic ...
            let (mut title_str, prefix, color) = match state.mode {
                InputMode::Searching => (" Search ".to_string(), "/ ", Color::Green),
                InputMode::Editing => (" Edit Title ".to_string(), "> ", Color::Magenta),
                InputMode::EditingDescription => {
                    (" Edit Description ".to_string(), "📝 ", Color::Blue)
                }
                InputMode::Creating => {
                    if state.creating_child_of.is_some() {
                        (" Create Child Task ".to_string(), "> ", Color::LightYellow)
                    } else {
                        (" Create Task ".to_string(), "> ", Color::Yellow)
                    }
                }
                _ => (" Create Task ".to_string(), "> ", Color::Yellow),
            };

            let show_tag_hint = (state.mode == InputMode::Searching
                && state.input_buffer.starts_with('#'))
                || (state.mode == InputMode::Creating
                    && state.input_buffer.starts_with('#')
                    && state.creating_child_of.is_none());

            if show_tag_hint {
                title_str.push_str(" [Enter to jump to tag] ");
            }

            let input_text = format!("{}{}", prefix, state.input_buffer);
            let input = Paragraph::new(input_text)
                .style(Style::default().fg(color))
                .block(Block::default().borders(Borders::ALL).title(title_str))
                .wrap(Wrap { trim: false });
            f.render_widget(input, footer_area);

            // Cursor rendering
            let cursor_x =
                footer_area.x + 1 + prefix.chars().count() as u16 + state.cursor_position as u16;
            f.set_cursor_position((
                cursor_x.min(footer_area.x + footer_area.width - 2),
                footer_area.y + 1,
            ));
        }
        _ => {
            if state.show_full_help {
                let p = Paragraph::new(full_help_text)
                    .block(Block::default().borders(Borders::ALL).title(" Help "))
                    .wrap(Wrap { trim: false });
                f.render_widget(p, footer_area);
            } else {
                let status = Paragraph::new(state.message.clone())
                    .style(Style::default().fg(Color::Cyan))
                    .block(
                        Block::default()
                            .borders(Borders::LEFT | Borders::TOP | Borders::BOTTOM)
                            .title(" Status "),
                    );
                let help_str = match state.active_focus {
                    Focus::Sidebar => "Ret:Select Space:Vis *:All Tab:Tasks".to_string(),
                    Focus::Main => "a:Add e:Edit Spc:Done d:Del /:Find".to_string(),
                };
                let help = Paragraph::new(help_str).alignment(Alignment::Right).block(
                    Block::default()
                        .borders(Borders::RIGHT | Borders::TOP | Borders::BOTTOM)
                        .title(" Actions "),
                );

                let chunks = Layout::default()
                    .direction(Direction::Horizontal)
                    .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
                    .split(footer_area);
                f.render_widget(status, chunks[0]);
                f.render_widget(help, chunks[1]);
            }
        }
    }

    // Popup logic for Move/Export (simplified)
    if state.mode == InputMode::Moving {
        let area = centered_rect(60, 50, f.area());
        let items: Vec<ListItem> = state
            .move_targets
            .iter()
            .map(|c| ListItem::new(c.name.as_str()))
            .collect();
        let popup = List::new(items)
            .block(Block::default().borders(Borders::ALL).title(" Move Task "))
            .highlight_style(Style::default().bg(Color::Blue));
        f.render_widget(Clear, area);
        f.render_stateful_widget(popup, area, &mut state.move_selection_state);
    }
}

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);
    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}
