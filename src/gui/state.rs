use crate::client::RustyClient;
use crate::model::{CalendarListEntry, Task as TodoTask};
use crate::store::TaskStore;
use std::collections::HashSet;

#[derive(Default, PartialEq, Clone, Copy, Debug)]
pub enum AppState {
    #[default]
    Loading,
    Onboarding,
    Active,
    Settings,
}

#[derive(Default, PartialEq, Clone, Copy, Debug)]
pub enum SidebarMode {
    #[default]
    Calendars,
    Categories,
}

pub struct GuiApp {
    pub state: AppState,

    // Data
    pub store: TaskStore,
    pub tasks: Vec<TodoTask>,
    pub calendars: Vec<CalendarListEntry>,
    pub client: Option<RustyClient>,

    // UI State
    pub sidebar_mode: SidebarMode,
    pub active_cal_href: Option<String>,
    pub selected_categories: HashSet<String>,
    pub match_all_categories: bool,

    // PREFERENCES
    pub hide_completed: bool,
    pub hide_completed_in_tags: bool,

    // Inputs
    pub input_value: String,
    pub description_value: String,
    pub search_value: String,
    pub editing_uid: Option<String>,
    pub expanded_tasks: HashSet<String>,

    pub loading: bool,
    pub error_msg: Option<String>,

    pub ob_url: String,
    pub ob_user: String,
    pub ob_pass: String,
    pub ob_default_cal: Option<String>,

    pub yanked_uid: Option<String>,
}

impl Default for GuiApp {
    fn default() -> Self {
        Self {
            state: AppState::Loading,
            store: TaskStore::new(),
            tasks: vec![],
            calendars: vec![],
            client: None,
            sidebar_mode: SidebarMode::Calendars,
            active_cal_href: None,
            selected_categories: HashSet::new(),
            match_all_categories: false,

            // Default prefs
            hide_completed: false,
            hide_completed_in_tags: false,

            input_value: String::new(),
            description_value: String::new(),
            search_value: String::new(),
            editing_uid: None,
            expanded_tasks: HashSet::new(),
            loading: true,
            error_msg: None,
            ob_url: String::new(),
            ob_user: String::new(),
            ob_pass: String::new(),
            ob_default_cal: None,
            yanked_uid: None,
        }
    }
}
