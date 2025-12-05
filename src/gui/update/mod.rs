// File: ./src/gui/update/mod.rs
pub mod common;
pub mod network;
pub mod settings;
pub mod tasks;
pub mod view;

use crate::gui::message::Message;
use crate::gui::state::GuiApp;
use iced::Task;

pub fn update(app: &mut GuiApp, message: Message) -> Task<Message> {
    match message {
        Message::FontLoaded(_) => Task::none(),
        Message::DeleteComplete(_) => Task::none(),

        Message::ConfigLoaded(_)
        | Message::ObUrlChanged(_)
        | Message::ObUserChanged(_)
        | Message::ObPassChanged(_)
        | Message::ObDefaultCalChanged(_)
        | Message::ObInsecureToggled(_)
        | Message::ObSubmit
        | Message::OpenSettings
        | Message::CancelSettings
        | Message::ObSubmitOffline
        | Message::AliasKeyInput(_)
        | Message::AliasValueInput(_)
        | Message::AddAlias
        | Message::RemoveAlias(_)
        | Message::ObSortMonthsChanged(_) => settings::handle(app, message),

        Message::InputChanged(_)
        | Message::DescriptionChanged(_)
        | Message::StartCreateChild(_)
        | Message::SubmitTask
        | Message::ToggleTask(_, _)
        | Message::EditTaskStart(_)
        | Message::CancelEdit
        | Message::DeleteTask(_)
        | Message::ChangePriority(_, _)
        | Message::SetTaskStatus(_, _)
        | Message::YankTask(_)
        | Message::ClearYank
        | Message::MakeChild(_)
        | Message::RemoveParent(_)
        | Message::RemoveDependency(_, _)
        | Message::AddDependency(_)
        | Message::MoveTask(_, _)
        | Message::MigrateLocalTo(_) => tasks::handle(app, message),

        Message::TabPressed(_)
        | Message::DismissError
        | Message::ToggleAllCalendars(_)
        | Message::ToggleCalendarVisibility(_, _)
        | Message::IsolateCalendar(_)
        | Message::SidebarModeChanged(_)
        | Message::CategoryToggled(_)
        | Message::ClearAllTags
        | Message::CategoryMatchModeChanged(_)
        | Message::ToggleHideCompleted(_)
        | Message::ToggleHideFullyCompletedTags(_)
        | Message::SelectCalendar(_)
        | Message::ToggleCalendarDisabled(_, _)
        | Message::SearchChanged(_)
        | Message::SetMinDuration(_)
        | Message::SetMaxDuration(_)
        | Message::ToggleIncludeUnsetDuration(_)
        | Message::ToggleDetails(_)
        | Message::OpenHelp
        | Message::CloseHelp
        | Message::WindowDragged
        | Message::MinimizeWindow
        | Message::CloseWindow
        | Message::ResizeStart(_)
        | Message::ResizeUpdate(_)
        | Message::ResizeEnd
        | Message::WindowResized(_)
        | Message::WindowMoved(_) => view::handle(app, message),

        Message::Refresh
        | Message::Loaded(_)
        | Message::RefreshedAll(_)
        | Message::TasksRefreshed(_)
        | Message::SyncSaved(_)
        | Message::SyncToggleComplete(_)
        | Message::TaskMoved(_)
        | Message::MigrationComplete(_) => network::handle(app, message),
    }
}
