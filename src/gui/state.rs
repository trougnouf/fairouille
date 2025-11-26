use crate::client::RustyClient;
use crate::model::{CalendarListEntry, Task as TodoTask};
use crate::store::TaskStore;
use std::collections::{HashMap, HashSet};

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
    pub tag_aliases: HashMap<String, Vec<String>>,

    // UI State
    pub sidebar_mode: SidebarMode,
    pub active_cal_href: Option<String>,
    pub selected_categories: HashSet<String>,
    pub match_all_categories: bool,
    pub yanked_uid: Option<String>,

    // Preferences
    pub hide_completed: bool,
    pub hide_fully_completed_tags: bool,
    pub sort_cutoff_months: Option<u32>,

    // Inputs - Main
    pub input_value: String,
    pub description_value: String,
    pub search_value: String,
    pub editing_uid: Option<String>,
    pub expanded_tasks: HashSet<String>,

    // Inputs - Settings (Aliases)
    pub alias_input_key: String,
    pub alias_input_values: String,

    // System
    pub loading: bool,
    pub error_msg: Option<String>,

    // Onboarding / Config
    pub ob_url: String,
    pub ob_user: String,
    pub ob_pass: String,
    pub ob_default_cal: Option<String>,
    pub ob_sort_months_input: String,
}

impl Default for GuiApp {
    fn default() -> Self {
        Self {
            state: AppState::Loading,
            store: TaskStore::new(),
            tasks: vec![],
            calendars: vec![],
            client: None,
            tag_aliases: HashMap::new(),

            sidebar_mode: SidebarMode::Calendars,
            active_cal_href: None,
            selected_categories: HashSet::new(),
            match_all_categories: false,
            yanked_uid: None,

            hide_completed: false,
            hide_fully_completed_tags: true,
            sort_cutoff_months: Some(6),
            ob_sort_months_input: "6".to_string(),

            input_value: String::new(),
            description_value: String::new(),
            search_value: String::new(),
            editing_uid: None,
            expanded_tasks: HashSet::new(),

            alias_input_key: String::new(),
            alias_input_values: String::new(),

            loading: true,
            error_msg: None,
            ob_url: String::new(),
            ob_user: String::new(),
            ob_pass: String::new(),
            ob_default_cal: None,
        }
    }
}
