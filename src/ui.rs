use crate::model::{CalendarListEntry, Task};
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
};

// Tracks which pane has keyboard focus
#[derive(PartialEq)]
pub enum Focus {
    Sidebar,
    Main,
}

pub struct AppState {
    // Data
    pub tasks: Vec<Task>,
    pub calendars: Vec<CalendarListEntry>,

    // State
    pub list_state: ListState, // Task list
    pub cal_state: ListState,  // Calendar list
    pub active_focus: Focus,   // Which side is active?

    pub message: String,
    pub loading: bool,

    // Input
    pub show_input: bool,
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
            calendars: vec![],
            list_state: l_state,
            cal_state: c_state,
            active_focus: Focus::Main, // Start in task list
            message: "Tab: Switch View | Enter: Select Calendar".to_string(),
            loading: true,
            show_input: false,
            input_buffer: String::new(),
        }
    }

    // Generic helper to move any list
    fn move_list(state: &mut ListState, count: usize, delta: isize) {
        if count == 0 {
            return;
        }
        let current = state.selected().unwrap_or(0);

        let new_index = if delta > 0 {
            // Next
            if current >= count - 1 { 0 } else { current + 1 }
        } else {
            // Prev
            if current == 0 { count - 1 } else { current - 1 }
        };
        state.select(Some(new_index));
    }

    // --- Navigation Handlers ---
    pub fn next(&mut self) {
        match self.active_focus {
            Focus::Main => Self::move_list(&mut self.list_state, self.tasks.len(), 1),
            Focus::Sidebar => Self::move_list(&mut self.cal_state, self.calendars.len(), 1),
        }
    }

    pub fn previous(&mut self) {
        match self.active_focus {
            Focus::Main => Self::move_list(&mut self.list_state, self.tasks.len(), -1),
            Focus::Sidebar => Self::move_list(&mut self.cal_state, self.calendars.len(), -1),
        }
    }

    pub fn toggle_focus(&mut self) {
        self.active_focus = match self.active_focus {
            Focus::Main => Focus::Sidebar,
            Focus::Sidebar => Focus::Main,
        }
    }

    // (Keep jump_forward/backward for Main only for now for simplicity)
    pub fn jump_forward(&mut self, step: usize) {
        if self.active_focus == Focus::Main && !self.tasks.is_empty() {
            let c = self.list_state.selected().unwrap_or(0);
            let n = (c + step).min(self.tasks.len() - 1);
            self.list_state.select(Some(n));
        }
    }
    pub fn jump_backward(&mut self, step: usize) {
        if self.active_focus == Focus::Main && !self.tasks.is_empty() {
            let c = self.list_state.selected().unwrap_or(0);
            let n = c.saturating_sub(step);
            self.list_state.select(Some(n));
        }
    }
}

pub fn draw(f: &mut Frame, state: &mut AppState) {
    // 1. Vertical Split (Body / Footer)
    let v_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0), Constraint::Length(3)].as_ref())
        .split(f.area());

    // 2. Horizontal Split (Sidebar / Main)
    let h_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(25), // Sidebar width
            Constraint::Percentage(75), // Main width
        ])
        .split(v_chunks[0]);

    // --- RENDER SIDEBAR (Calendars) ---
    let cal_items: Vec<ListItem> = state
        .calendars
        .iter()
        .map(|c| ListItem::new(Line::from(c.name.as_str())))
        .collect();

    let sidebar_border_style = if state.active_focus == Focus::Sidebar {
        Style::default().fg(Color::Yellow) // Highlight border when focused
    } else {
        Style::default()
    };

    let sidebar = List::new(cal_items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Calendars ")
                .border_style(sidebar_border_style),
        )
        .highlight_style(
            Style::default()
                .add_modifier(Modifier::BOLD)
                .bg(Color::Blue),
        );

    f.render_stateful_widget(sidebar, h_chunks[0], &mut state.cal_state);

    // --- RENDER MAIN (Tasks) ---
    let task_items: Vec<ListItem> = state
        .tasks
        .iter()
        .map(|t| {
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
            let summary = format!("{} {}{}", checkbox, t.summary, due_str);
            ListItem::new(Line::from(vec![Span::styled(summary, style)]))
        })
        .collect();

    let main_border_style = if state.active_focus == Focus::Main {
        Style::default().fg(Color::Yellow)
    } else {
        Style::default()
    };

    let title = if state.loading {
        " Tasks (Loading...) "
    } else {
        " Tasks "
    };
    let task_list = List::new(task_items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(title)
                .border_style(main_border_style),
        )
        .highlight_style(
            Style::default()
                .add_modifier(Modifier::BOLD)
                .bg(Color::DarkGray),
        );

    f.render_stateful_widget(task_list, h_chunks[1], &mut state.list_state);

    // --- RENDER FOOTER ---
    if state.show_input {
        let input = Paragraph::new(format!("> {}_", state.input_buffer))
            .style(Style::default().fg(Color::Yellow))
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(" Create Task "),
            );
        f.render_widget(input, v_chunks[1]);
    } else {
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

        // Update Shortcuts
        let help_text = "Tab: View | Enter: Load Cal | a: Add | Space: Done";
        let help = Paragraph::new(help_text)
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
