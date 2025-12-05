// File: ./src/gui/message.rs
use crate::client::RustyClient;
use crate::config::Config;
use crate::gui::state::{ResizeDirection, SidebarMode};
use crate::model::{CalendarListEntry, Task as TodoTask};
use iced::widget::text_editor;

pub type LoadedResult = Result<
    (
        RustyClient,
        Vec<CalendarListEntry>,
        Vec<TodoTask>,
        Option<String>,
        Option<String>,
    ),
    String,
>;

#[derive(Debug, Clone)]
pub enum Message {
    ObUrlChanged(String),
    ObUserChanged(String),
    ObPassChanged(String),
    ObInsecureToggled(bool),
    ToggleCalendarVisibility(String, bool),
    ToggleCalendarDisabled(String, bool),
    ObDefaultCalChanged(String),
    ObSubmit,
    OpenSettings,
    CancelSettings,
    OpenHelp,
    CloseHelp,
    InputChanged(String),

    DescriptionChanged(text_editor::Action),

    SearchChanged(String),
    SubmitTask,
    ToggleTask(usize, bool),
    DeleteTask(usize),
    EditTaskStart(usize),
    CancelEdit,
    ChangePriority(usize, i8),
    SetTaskStatus(usize, crate::model::TaskStatus),
    SetMinDuration(Option<u32>),
    SetMaxDuration(Option<u32>),
    ToggleIncludeUnsetDuration(bool),
    ToggleDetails(String),
    ConfigLoaded(Result<Config, String>),
    ObSortMonthsChanged(String),

    Loaded(LoadedResult),
    Refresh,

    SyncSaved(Result<TodoTask, String>),
    SyncToggleComplete(Box<Result<(TodoTask, Option<TodoTask>), String>>),

    TasksRefreshed(Result<(String, Vec<TodoTask>), String>),
    DeleteComplete(#[allow(dead_code)] Result<(), String>),

    SidebarModeChanged(SidebarMode),
    SelectCalendar(String),
    IsolateCalendar(String),
    CategoryToggled(String),
    ClearAllTags,
    CategoryMatchModeChanged(bool),
    RefreshedAll(Result<Vec<(String, Vec<TodoTask>)>, String>),

    ToggleHideCompleted(bool),
    ToggleHideFullyCompletedTags(bool),

    YankTask(String),
    ClearYank,
    StartCreateChild(String),
    AddDependency(String),
    MakeChild(String),
    RemoveParent(String),
    RemoveDependency(String, String),

    AliasKeyInput(String),
    AliasValueInput(String),
    AddAlias,
    RemoveAlias(String),
    MoveTask(String, String),

    TaskMoved(Result<TodoTask, String>),
    ObSubmitOffline,
    MigrateLocalTo(String),

    MigrationComplete(Result<usize, String>),
    FontLoaded(Result<(), String>),
    DismissError,
    ToggleAllCalendars(bool),

    TabPressed(bool),

    // Window Controls
    WindowDragged,
    MinimizeWindow,
    CloseWindow,
    WindowResized(iced::Size),
    WindowMoved(iced::Point), // Added

    // Resize
    ResizeStart(ResizeDirection),
    ResizeUpdate(iced::Point),
    ResizeEnd,
}
