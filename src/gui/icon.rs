// File: ./src/gui/icon.rs
use iced::Font;
use iced::widget::{Text, text};

pub const FONT_BYTES: &[u8] = include_bytes!("../../assets/fonts/SymbolsNerdFont-Regular.ttf");
pub const FONT: Font = Font::with_name("Symbols Nerd Font");

// Load the Logo
pub const LOGO: &[u8] = include_bytes!("../../assets/cfait.svg");

pub fn icon<'a>(codepoint: char) -> Text<'a> {
    text(codepoint.to_string()).font(FONT)
}

// --- NERD FONT MAPPING ---
pub const CALENDAR: char = '\u{f073}'; // 
pub const TAG: char = '\u{f02b}'; // 
pub const SETTINGS: char = '\u{f013}'; // 
pub const REFRESH: char = '\u{f0450}'; // nf-md-refresh
pub const UNSYNCED: char = '\u{f0c2}'; //  (Cloud)
pub const PLUS: char = '\u{f0603}'; // nf-md-priority_high
pub const MINUS: char = '\u{f0604}'; // nf-md-priority_low
pub const TRASH: char = '\u{f1f8}'; // 
pub const CHECK: char = '\u{f00c}'; // 
pub const CROSS: char = '\u{f00d}'; // 
pub const EDIT: char = '\u{f040}'; // 
pub const PLAY: char = '\u{eb2c}'; // nf-cod-play
pub const PLAY_FA: char = '\u{f04b}'; // nf-fa-play
pub const PAUSE: char = '\u{f04c}'; // 
pub const STOP: char = '\u{f04d}'; // 
pub const LOCK: char = '\u{f023}'; // 
pub const LINK: char = '\u{f0c1}'; // 
pub const UNLINK: char = '\u{f127}'; // 
pub const SHIELD: char = '\u{f32a}'; // 
pub const CHILD_ARROW: char = '\u{f149}'; // 
pub const INFO: char = '\u{f129}'; // 
pub const REPEAT: char = '\u{f0b6}'; // 
pub const ARROW_RIGHT: char = '\u{f061}'; // 
pub const CHECK_SQUARE: char = '\u{f14a}'; // 
pub const SQUARE: char = '\u{f096}'; // 
pub const EXPORT: char = '\u{f56e}'; // 
pub const BLOCKED: char = '\u{f479}'; // nf-oct-blocked
pub const CHILD: char = '\u{f0a89}'; // nf-md-account_child
pub const CREATE_CHILD: char = '\u{f0014}'; // nf-md-account_plus
pub const CLEAR_ALL: char = '\u{eabf}'; // nf-cod-clear_all

// New Icons
pub const SETTINGS_GEAR: char = '\u{e690}'; // nf-seti-settings
pub const HELP_RHOMBUS: char = '\u{f0625}'; // nf-md-help_circle_outline

// Window Controls
pub const WINDOW_MINIMIZE: char = '\u{f2d1}'; // nf-fa-window_minimize
