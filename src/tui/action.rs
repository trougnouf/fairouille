use crate::model::{CalendarListEntry, Task};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SidebarMode {
    Calendars,
    Categories,
}

#[derive(Debug)]
pub enum Action {
    SwitchCalendar(String),

    CreateTask(Task),

    UpdateTask(Task),
    ToggleTask(Task),
    MarkInProcess(Task),
    MarkCancelled(Task),
    DeleteTask(Task),
    Refresh,
    Quit,
    MoveTask(Task, String), // Task, New Calendar Href
}

#[derive(Debug)]
pub enum AppEvent {
    CalendarsLoaded(Vec<CalendarListEntry>),
    TasksLoaded(Vec<(String, Vec<Task>)>),
    Error(String),
    Status(String),
}
