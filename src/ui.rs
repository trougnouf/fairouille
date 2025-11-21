use crate::model::{CalendarListEntry, Task};
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
};

#[derive(PartialEq)]
pub enum Focus {
    Sidebar,
    Main,
}

#[derive(PartialEq)]
pub enum InputMode {
    Normal,
    Creating,
    Searching,
}

pub struct AppState {
    // Data
    pub tasks: Vec<Task>,
    pub view_indices: Vec<usize>, // Indices of tasks currently visible (filtered)
    pub calendars: Vec<CalendarListEntry>,

    // State
    pub list_state: ListState,
    pub cal_state: ListState,
    pub active_focus: Focus,
    pub message: String,
    pub loading: bool,

    // Input
    pub mode: InputMode,
    pub input_buffer: String,
}

impl AppState {
    pub fn new() -> Self {
        let mut l_state = ListState::default();
        l_state.select(Some(0));
        let mut c_state = ListState::default();
        c_state.select(Some(0));

        Self {
            tasks: vec![],
            view_indices: vec![],
            calendars: vec![],
            list_state: l_state,
            cal_state: c_state,
            active_focus: Focus::Main,
            message: "Tab: View | /: Search | a: Add".to_string(),
            loading: true,
            mode: InputMode::Normal,
            input_buffer: String::new(),
        }
    }

    pub fn recalculate_view(&mut self) {
        if self.mode == InputMode::Searching && !self.input_buffer.is_empty() {
            let query = self.input_buffer.to_lowercase();
            self.view_indices = self
                .tasks
                .iter()
                .enumerate()
                .filter(|(_, t)| t.summary.to_lowercase().contains(&query))
                .map(|(i, _)| i)
                .collect();
        } else {
            self.view_indices = (0..self.tasks.len()).collect();
        }

        // Fix selection bounds
        let sel = self.list_state.selected().unwrap_or(0);
        if self.view_indices.is_empty() {
            self.list_state.select(Some(0));
        } else if sel >= self.view_indices.len() {
            self.list_state.select(Some(self.view_indices.len() - 1));
        }
    }

    pub fn get_selected_master_index(&self) -> Option<usize> {
        if let Some(view_idx) = self.list_state.selected() {
            if view_idx < self.view_indices.len() {
                return Some(self.view_indices[view_idx]);
            }
        }
        None
    }

    // --- Navigation ---

    pub fn next(&mut self) {
        match self.active_focus {
            Focus::Main => {
                let len = self.view_indices.len();
                if len == 0 {
                    return;
                }
                let i = match self.list_state.selected() {
                    Some(i) => {
                        if i >= len - 1 {
                            0
                        } else {
                            i + 1
                        }
                    }
                    None => 0,
                };
                self.list_state.select(Some(i));
            }
            Focus::Sidebar => {
                let len = self.calendars.len();
                if len == 0 {
                    return;
                }
                let i = match self.cal_state.selected() {
                    Some(i) => {
                        if i >= len - 1 {
                            0
                        } else {
                            i + 1
                        }
                    }
                    None => 0,
                };
                self.cal_state.select(Some(i));
            }
        }
    }

    pub fn previous(&mut self) {
        match self.active_focus {
            Focus::Main => {
                let len = self.view_indices.len();
                if len == 0 {
                    return;
                }
                let i = match self.list_state.selected() {
                    Some(i) => {
                        if i == 0 {
                            len - 1
                        } else {
                            i - 1
                        }
                    }
                    None => 0,
                };
                self.list_state.select(Some(i));
            }
            Focus::Sidebar => {
                let len = self.calendars.len();
                if len == 0 {
                    return;
                }
                let i = match self.cal_state.selected() {
                    Some(i) => {
                        if i == 0 {
                            len - 1
                        } else {
                            i - 1
                        }
                    }
                    None => 0,
                };
                self.cal_state.select(Some(i));
            }
        }
    }

    pub fn jump_forward(&mut self, step: usize) {
        match self.active_focus {
            Focus::Main => {
                if self.view_indices.is_empty() {
                    return;
                }
                let current = self.list_state.selected().unwrap_or(0);
                let new_index = (current + step).min(self.view_indices.len() - 1);
                self.list_state.select(Some(new_index));
            }
            Focus::Sidebar => {
                if self.calendars.is_empty() {
                    return;
                }
                let current = self.cal_state.selected().unwrap_or(0);
                let new_index = (current + step).min(self.calendars.len() - 1);
                self.cal_state.select(Some(new_index));
            }
        }
    }

    pub fn jump_backward(&mut self, step: usize) {
        match self.active_focus {
            Focus::Main => {
                if self.view_indices.is_empty() {
                    return;
                }
                let current = self.list_state.selected().unwrap_or(0);
                let new_index = current.saturating_sub(step);
                self.list_state.select(Some(new_index));
            }
            Focus::Sidebar => {
                if self.calendars.is_empty() {
                    return;
                }
                let current = self.cal_state.selected().unwrap_or(0);
                let new_index = current.saturating_sub(step);
                self.cal_state.select(Some(new_index));
            }
        }
    }

    pub fn toggle_focus(&mut self) {
        self.active_focus = match self.active_focus {
            Focus::Main => Focus::Sidebar,
            Focus::Sidebar => Focus::Main,
        }
    }
}

pub fn draw(f: &mut Frame, state: &mut AppState) {
    let v_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0), Constraint::Length(3)].as_ref())
        .split(f.area());

    let h_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(25), Constraint::Percentage(75)])
        .split(v_chunks[0]);

    // Sidebar
    let cal_items: Vec<ListItem> = state
        .calendars
        .iter()
        .map(|c| ListItem::new(Line::from(c.name.as_str())))
        .collect();
    let sidebar_style = if state.active_focus == Focus::Sidebar {
        Style::default().fg(Color::Yellow)
    } else {
        Style::default()
    };
    let sidebar = List::new(cal_items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Calendars ")
                .border_style(sidebar_style),
        )
        .highlight_style(
            Style::default()
                .add_modifier(Modifier::BOLD)
                .bg(Color::Blue),
        );
    f.render_stateful_widget(sidebar, h_chunks[0], &mut state.cal_state);

    // Main List (Render using VIEW INDICES)
    let task_items: Vec<ListItem> = state
        .view_indices
        .iter()
        .map(|&idx| {
            let t = &state.tasks[idx];
            let style = match t.priority {
                1..=4 => Style::default().fg(Color::Red),
                5 => Style::default().fg(Color::Yellow),
                _ => Style::default().fg(Color::White),
            };
            let checkbox = if t.completed { "[x]" } else { "[ ]" };
            let due_str = match t.due {
                Some(d) => format!(" ({})", d.format("%d/%m")),
                None => "".to_string(),
            };
            ListItem::new(Line::from(vec![Span::styled(
                format!("{} {}{}", checkbox, t.summary, due_str),
                style,
            )]))
        })
        .collect();

    let main_style = if state.active_focus == Focus::Main {
        Style::default().fg(Color::Yellow)
    } else {
        Style::default()
    };

    // FIX: Return String in both branches
    let title = if state.loading {
        " Tasks (Loading...) ".to_string()
    } else {
        format!(" Tasks ({}) ", state.view_indices.len())
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
                .bg(Color::DarkGray),
        );
    f.render_stateful_widget(task_list, h_chunks[1], &mut state.list_state);

    // Footer
    match state.mode {
        InputMode::Creating => {
            let input = Paragraph::new(format!("> {}_", state.input_buffer))
                .style(Style::default().fg(Color::Yellow))
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title(" Create Task "),
                );
            f.render_widget(input, v_chunks[1]);
        }
        InputMode::Searching => {
            let input = Paragraph::new(format!("/ {}_", state.input_buffer))
                .style(Style::default().fg(Color::Green))
                .block(Block::default().borders(Borders::ALL).title(" Search "));
            f.render_widget(input, v_chunks[1]);
        }
        InputMode::Normal => {
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

            let help = Paragraph::new("Tab:View | Enter:Sel | /:Find | a:Add | Space:Done")
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
}
