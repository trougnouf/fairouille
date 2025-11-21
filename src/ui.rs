use crate::model::{CalendarListEntry, Task};
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Wrap}, // Added Wrap
};

// ... Focus, InputMode, AppState structs/impls remain exactly the same ...
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
    Editing,
}

pub struct AppState {
    pub tasks: Vec<Task>,
    pub view_indices: Vec<usize>,
    pub calendars: Vec<CalendarListEntry>,
    pub list_state: ListState,
    pub cal_state: ListState,
    pub active_focus: Focus,
    pub message: String,
    pub loading: bool,
    pub mode: InputMode,
    pub input_buffer: String,
    pub cursor_position: usize,
    pub editing_index: Option<usize>,
}

impl AppState {
    // ... same methods as before ...
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
            message: "Tab: View | /: Search | a: Add | e: Edit".to_string(),
            loading: true,
            mode: InputMode::Normal,
            input_buffer: String::new(),
            cursor_position: 0,
            editing_index: None,
        }
    }

    // ... copy helper methods (move_cursor, etc) from previous version ...
    pub fn move_cursor_left(&mut self) {
        let cursor_moved_left = self.cursor_position.saturating_sub(1);
        self.cursor_position = self.clamp_cursor(cursor_moved_left);
    }
    pub fn move_cursor_right(&mut self) {
        let cursor_moved_right = self.cursor_position.saturating_add(1);
        self.cursor_position = self.clamp_cursor(cursor_moved_right);
    }
    pub fn enter_char(&mut self, new_char: char) {
        self.input_buffer.insert(self.cursor_position, new_char);
        self.move_cursor_right();
    }
    pub fn delete_char(&mut self) {
        if self.cursor_position != 0 {
            let current_index = self.cursor_position;
            let from_left_to_current_index = current_index - 1;
            let before_char_to_delete = self.input_buffer.chars().take(from_left_to_current_index);
            let after_char_to_delete = self.input_buffer.chars().skip(current_index);
            self.input_buffer = before_char_to_delete.chain(after_char_to_delete).collect();
            self.move_cursor_left();
        }
    }
    pub fn reset_input(&mut self) {
        self.input_buffer.clear();
        self.cursor_position = 0;
    }
    fn clamp_cursor(&self, new_cursor_pos: usize) -> usize {
        new_cursor_pos.clamp(0, self.input_buffer.chars().count())
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
    // Layout:
    // Top: Body
    // Bottom: Footer
    let v_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0), Constraint::Length(3)].as_ref())
        .split(f.area());

    // Body: Sidebar | Main Area
    let h_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(25), Constraint::Percentage(75)])
        .split(v_chunks[0]);

    // Main Area: Task List | Details Pane (Vertical split)
    let main_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(70), Constraint::Percentage(30)])
        .split(h_chunks[1]);

    // --- SIDEBAR ---
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

    // --- MAIN: LIST ---
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
            let indent = "  ".repeat(t.depth);
            let summary = format!("{}{} {}{}", indent, checkbox, t.summary, due_str);
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
    f.render_stateful_widget(task_list, main_chunks[0], &mut state.list_state);

    // --- MAIN: DETAILS ---
    let details_text = if let Some(idx) = state.get_selected_master_index() {
        let task = &state.tasks[idx];
        if task.description.is_empty() {
            "No description.".to_string()
        } else {
            task.description.clone()
        }
    } else {
        "".to_string()
    };

    let details = Paragraph::new(details_text)
        .wrap(Wrap { trim: true })
        .block(Block::default().borders(Borders::ALL).title(" Details "));

    f.render_widget(details, main_chunks[1]);

    // --- FOOTER ---
    let footer_area = v_chunks[1];
    match state.mode {
        InputMode::Creating | InputMode::Editing | InputMode::Searching => {
            let (title, prefix, color) = match state.mode {
                InputMode::Searching => (" Search ", "/ ", Color::Green),
                InputMode::Editing => (" Edit Task ", "> ", Color::Magenta),
                _ => (" Create Task ", "> ", Color::Yellow),
            };
            let input = Paragraph::new(format!("{}{}", prefix, state.input_buffer))
                .style(Style::default().fg(color))
                .block(Block::default().borders(Borders::ALL).title(title));
            f.render_widget(input, footer_area);
            let cursor_x = footer_area.x + 1 + prefix.len() as u16 + state.cursor_position as u16;
            let cursor_y = footer_area.y + 1;
            f.set_cursor_position((cursor_x, cursor_y));
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
            let help = Paragraph::new("Tab:View | /:Find | a:Add | e:Edit | d:Del | >/<:Indent")
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
