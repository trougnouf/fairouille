use crate::client::RustyClient;
use crate::model::{CalendarListEntry, Task as TodoTask};
use std::collections::HashSet;

#[derive(Default)]
pub enum AppState {
    #[default]
    Loading,
    Onboarding,
    Active,
    Settings,
}

pub struct GuiApp {
    pub state: AppState,
    pub ob_url: String,
    pub ob_user: String,
    pub ob_pass: String,
    pub ob_default_cal: Option<String>,
    pub tasks: Vec<TodoTask>,
    pub calendars: Vec<CalendarListEntry>,
    pub active_cal_href: Option<String>,
    pub input_value: String,
    pub description_value: String,
    pub search_value: String,
    pub editing_uid: Option<String>,
    pub expanded_tasks: HashSet<String>,
    pub client: Option<RustyClient>,
    pub loading: bool,
    pub error_msg: Option<String>,
}

impl Default for GuiApp {
    fn default() -> Self {
        Self {
            state: AppState::Loading,
            ob_url: String::new(),
            ob_user: String::new(),
            ob_pass: String::new(),
            ob_default_cal: None,
            tasks: vec![],
            calendars: vec![],
            active_cal_href: None,
            input_value: String::new(),
            description_value: String::new(),
            search_value: String::new(),
            editing_uid: None,
            expanded_tasks: HashSet::new(),
            client: None,
            loading: true,
            error_msg: None,
        }
    }
}
