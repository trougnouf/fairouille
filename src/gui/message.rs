use crate::client::RustyClient;
use crate::config::Config;
use crate::gui::state::SidebarMode;
use crate::model::{CalendarListEntry, Task as TodoTask};

#[derive(Debug, Clone)]
pub enum Message {
    ObUrlChanged(String),
    ObUserChanged(String),
    ObPassChanged(String),
    ObDefaultCalChanged(String),
    ObSubmit,
    OpenSettings,
    CancelSettings,
    InputChanged(String),
    DescriptionChanged(String),
    SearchChanged(String),
    SubmitTask,
    ToggleTask(usize, bool),
    DeleteTask(usize),
    EditTaskStart(usize),
    CancelEdit,
    ChangePriority(usize, i8),
    IndentTask(usize),
    OutdentTask(usize),
    ToggleDetails(String),
    ConfigLoaded(Result<Config, String>),
    Loaded(
        Result<
            (
                RustyClient,
                Vec<CalendarListEntry>,
                Vec<TodoTask>,
                Option<String>,
            ),
            String,
        >,
    ),
    SyncSaved(Result<TodoTask, String>),
    SyncToggleComplete(Result<(TodoTask, Option<TodoTask>), String>),
    TasksRefreshed(Result<Vec<TodoTask>, String>),
    DeleteComplete(#[allow(dead_code)] Result<(), String>),

    SidebarModeChanged(SidebarMode),
    SelectCalendar(String),
    CategoryToggled(String),
    CategoryMatchModeChanged(bool),
    RefreshedAll(Result<Vec<(String, Vec<TodoTask>)>, String>),

    ToggleHideCompleted(bool),
    ToggleHideCompletedInTags(bool),
    YankTask(String),
    ClearYank, // NEW: To un-select
    AddDependency(String),
    MakeChild(String), // NEW: To set parent_uid
}
