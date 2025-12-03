use iced::Font;
use iced::widget::{Text, text};

// Embed the font bytes directly into the binary.
// Embed the real font bytes directly into the binary. The font file must exist
// at `assets/fonts/SymbolsNerdFont-Regular.ttf` (you mentioned you've saved it).
pub const FONT_BYTES: &[u8] = include_bytes!("../../assets/fonts/SymbolsNerdFont-Regular.ttf");

// The identifier used by Iced to look up the font after loading
pub const FONT: Font = Font::with_name("Symbols Nerd Font");

// Helper to create an Icon widget
pub fn icon<'a>(codepoint: char) -> Text<'a> {
    text(codepoint.to_string()).font(FONT)
}

// --- NERD FONT MAPPING ---
// We map readable names to the specific unicode points in the Nerd Font file.
// These are standard Nerd Font / FontAwesome codes.

pub const CALENDAR: char = '\u{f073}'; // 
pub const TAG: char = '\u{f02b}'; // 
pub const SETTINGS: char = '\u{f013}'; // 
// Use MaterialDesign calendar_refresh glyph from Nerd Font MD set
pub const REFRESH: char = '\u{f01e1}'; // nf-md-calendar_refresh
pub const UNSYNCED: char = '\u{f0c2}'; //  (Cloud)
// Use MaterialDesign priority icons for clearer semantics
pub const PLUS: char = '\u{f0603}'; // nf-md-priority_high (f0603)
pub const MINUS: char = '\u{f0604}'; // nf-md-priority_low (f0604)
pub const TRASH: char = '\u{f1f8}'; // 
pub const CHECK: char = '\u{f00c}'; // 
pub const CROSS: char = '\u{f00d}'; // 
// Use a standard pencil/edit glyph that's widely present in nerd fonts
pub const EDIT: char = '\u{f040}'; //  (Pencil)
// Prefer the patched codicon play glyph from NF (nf-cod-play)
// Codicon play (used on the right action button)
pub const PLAY: char = '\u{eb2c}'; // nf-cod-play (eb2c)
// FontAwesome play (used inside the status box for in-process tasks)
pub const PLAY_FA: char = '\u{f04b}'; // nf-fa-play (f04b)
pub const PAUSE: char = '\u{f04c}'; // 
pub const STOP: char = '\u{f04d}'; // 
pub const LOCK: char = '\u{f023}'; // 
pub const LINK: char = '\u{f0c1}'; // 
pub const UNLINK: char = '\u{f127}'; // 
pub const SHIELD: char = '\u{f32a}'; // 
pub const CHILD_ARROW: char = '\u{f149}'; // 
pub const INFO: char = '\u{f129}'; // 
pub const REPEAT: char = '\u{f01e}'; // 
pub const ARROW_RIGHT: char = '\u{f061}'; // 
pub const CHECK_SQUARE: char = '\u{f14a}'; // 
pub const SQUARE: char = '\u{f096}'; // 
pub const EXPORT: char = '\u{f56e}'; // 
pub const BLOCKED: char = '\u{f479}'; // nf-oct-blocked
pub const CHILD: char = '\u{f0a89}'; // nf-md-account_child
pub const CREATE_CHILD: char = '\u{f0014}'; // nf-md-account_plus
