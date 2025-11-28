use crate::storage::LOCAL_CALENDAR_HREF;
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
    let v_chunks = Layout::default()
        .direction(Direction::Vertical)
        // REMOVED .as_ref() below
        .constraints([Constraint::Min(0), Constraint::Length(3)])
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
                .filter(|c| !state.hidden_calendars.contains(&c.href))
                .map(|c| {
                    let prefix = if Some(&c.href) == state.active_cal_href.as_ref() {
                        "* "
                    } else {
                        "  "
                    };
                    ListItem::new(Line::from(format!("{}{}", prefix, c.name)))
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
                .map(|c| {
                    let selected = if state.selected_categories.contains(c) {
                        "[x]"
                    } else {
                        "[ ]"
                    };

                    let display_name = if c == UNCATEGORIZED_ID {
                        "Uncategorized".to_string()
                    } else {
                        format!("#{}", c)
                    };

                    ListItem::new(Line::from(format!("{} {}", selected, display_name)))
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
    let task_items: Vec<ListItem> = state
        .tasks
        .iter()
        .map(|t| {
            // Logic: Check if blocked
            let is_blocked = state.store.is_blocked(t);

            let style = if is_blocked {
                // Gray out blocked tasks
                Style::default().fg(Color::DarkGray)
            } else {
                match t.priority {
                    1..=4 => Style::default().fg(Color::Red),
                    5 => Style::default().fg(Color::Yellow),
                    _ => Style::default().fg(Color::White),
                }
            };

            let checkbox = match t.status {
                crate::model::TaskStatus::Completed => "[x]",
                crate::model::TaskStatus::Cancelled => "[-]",
                crate::model::TaskStatus::InProcess => "[>]",
                crate::model::TaskStatus::NeedsAction => "[ ]",
            };
            // Add [B] indicator if blocked
            let blocked_str = if is_blocked { " [B]" } else { "" };

            let due_str = match t.due {
                Some(d) => format!(" ({})", d.format("%d/%m")),
                None => "".to_string(),
            };

            let dur_str = if let Some(mins) = t.estimated_duration {
                if mins >= 525600 {
                    format!(" [~{}y]", mins / 525600)
                } else if mins >= 43200 {
                    format!(" [~{}mo]", mins / 43200)
                } else if mins >= 10080 {
                    format!(" [~{}w]", mins / 10080)
                } else if mins >= 1440 {
                    format!(" [~{}d]", mins / 1440)
                } else if mins >= 60 {
                    format!(" [~{}h]", mins / 60)
                } else {
                    format!(" [~{}m]", mins)
                }
            } else {
                "".to_string()
            };

            let show_indent = state.active_cal_href.is_some() && state.mode != InputMode::Searching;
            let indent = if show_indent {
                "  ".repeat(t.depth)
            } else {
                "".to_string()
            };

            let recur_str = if t.rrule.is_some() { " (R)" } else { "" };

            let mut cat_str = String::new();
            if !t.categories.is_empty() {
                cat_str = format!(" [{}]", t.categories.join(", "));
            }

            let summary = format!(
                "{}{} {}{}{}{}{}{}",
                indent, checkbox, t.summary, dur_str, due_str, recur_str, cat_str, blocked_str
            );
            ListItem::new(Line::from(vec![Span::styled(summary, style)]))
        })
        .collect();

    let main_style = if state.active_focus == Focus::Main {
        Style::default().fg(Color::Yellow)
    } else {
        Style::default()
    };

    let title = if state.loading {
        " Tasks (Loading...) ".to_string()
    } else {
        format!(" Tasks ({}) ", state.tasks.len())
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
                .bg(Color::Blue), // <--- Fixed color
        );
    f.render_stateful_widget(task_list, main_chunks[0], &mut state.list_state);

    // 2. SHOW DEPENDENCIES IN DETAILS (Text only)
    let mut full_details = String::new();

    if let Some(task) = state.get_selected_task() {
        // A. Description
        if !task.description.is_empty() {
            full_details.push_str(&task.description);
            full_details.push_str("\n\n");
        }

        // B. Dependencies
        if !task.dependencies.is_empty() {
            full_details.push_str("[Blocked By]:\n"); // <--- Text only
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

    // --- Footer ---
    let footer_area = v_chunks[1];
    match state.mode {
        InputMode::Creating
        | InputMode::Editing
        | InputMode::Searching
        | InputMode::EditingDescription => {
            let (title, prefix, color) = match state.mode {
                InputMode::Searching => (" Search ", "/ ", Color::Green),
                InputMode::Editing => (" Edit Title ", "> ", Color::Magenta),
                InputMode::EditingDescription => (" Edit Description ", "📝 ", Color::Blue),
                _ => (" Create Task ", "> ", Color::Yellow),
            };
            let input = Paragraph::new(format!("{}{}", prefix, state.input_buffer))
                .style(Style::default().fg(color))
                .block(Block::default().borders(Borders::ALL).title(title));
            f.render_widget(input, footer_area);
            let cursor_x =
                footer_area.x + 1 + prefix.chars().count() as u16 + state.cursor_position as u16;
            let cursor_y = footer_area.y + 1;
            f.set_cursor_position((cursor_x, cursor_y));
        }

        InputMode::Normal | InputMode::Moving | InputMode::Exporting => {
            let f_chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
                .split(v_chunks[1]);
            let status = Paragraph::new(state.message.clone())
                .style(Style::default().fg(Color::Cyan))
                .block(
                    Block::default()
                        .borders(Borders::LEFT | Borders::TOP | Borders::BOTTOM)
                        .title(" Status "),
                );

            let help_str = match state.active_focus {
                Focus::Sidebar => match state.sidebar_mode {
                    SidebarMode::Calendars => "Enter:Select | 2:Tags".to_string(),
                    SidebarMode::Categories => {
                        "Enter:Toggle | m:Match(AND/OR) | 1:Cals".to_string()
                    }
                },
                Focus::Main => {
                    let mut s =
                        "/:Find | a:Add | e:Edit | E:Desc | M:Move | d:Del | y:Yank | r:Sync"
                            .to_string();
                    // Conditionally show 'X:Export'
                    if state.active_cal_href.as_deref() == Some(LOCAL_CALENDAR_HREF) {
                        s.push_str(" | X:Export");
                    }
                    // Only show Block/Child if something is in the clipboard
                    if state.yanked_uid.is_some() {
                        s.push_str(" | b:Block | c:Child");
                    }
                    s.push_str(" | H:Hide");
                    s
                }
            };

            let help = Paragraph::new(help_str)
                .style(Style::default().fg(Color::DarkGray))
                .alignment(Alignment::Right)
                .block(
                    Block::default()
                        .borders(Borders::RIGHT | Borders::TOP | Borders::BOTTOM)
                        .title(" Actions "),
                );
            f.render_widget(status, f_chunks[0]);
            f.render_widget(help, f_chunks[1]);
        }
    }
    if state.mode == InputMode::Moving {
        let area = centered_rect(60, 50, f.area());

        let items: Vec<ListItem> = state
            .move_targets
            .iter()
            .map(|c| ListItem::new(c.name.as_str()))
            .collect();

        let popup_list = List::new(items)
            .block(
                Block::default()
                    .title(" Move task to... ")
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::Yellow)),
            )
            .highlight_style(
                Style::default()
                    .add_modifier(Modifier::BOLD)
                    .bg(Color::Blue),
            );

        // First render a `Clear` widget to erase the area behind the popup
        f.render_widget(Clear, area);
        // Then render the popup
        f.render_stateful_widget(popup_list, area, &mut state.move_selection_state);
    }
    if state.mode == InputMode::Exporting {
        let area = centered_rect(60, 50, f.area());
        let items: Vec<ListItem> = state
            .export_targets
            .iter()
            .map(|c| ListItem::new(c.name.as_str()))
            .collect();
        let popup = List::new(items)
            .block(
                Block::default()
                    .title(" Export all tasks to... ")
                    .borders(Borders::ALL),
            )
            .highlight_style(
                Style::default()
                    .add_modifier(Modifier::BOLD)
                    .bg(Color::Blue),
            );

        f.render_widget(Clear, area);
        f.render_stateful_widget(popup, area, &mut state.export_selection_state);
    }
}

/// Helper function to create a centered rect using up certain percentages of the available rect.
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
