// File: src/gui/view/help.rs
use crate::gui::message::Message;
use iced::widget::{button, column, container, row, scrollable, text, Space};
use iced::{Color, Element, Length, Theme};

// --- STYLE CONSTANTS ---
const COL_ACCENT: Color = Color::from_rgb(0.4, 0.7, 1.0); // Soft Blue
const COL_SYNTAX: Color = Color::from_rgb(1.0, 0.85, 0.4); // Gold/Yellow
const COL_MUTED: Color = Color::from_rgb(0.6, 0.6, 0.6); // Grey
const COL_CARD_BG: Color = Color::from_rgb(0.15, 0.15, 0.17); // Slightly lighter than pure black

pub fn view_help() -> Element<'static, Message> {
    let title = row![
        crate::gui::icon::icon(crate::gui::icon::HELP_RHOMBUS).size(28).style(|_: &Theme| text::Style { color: Some(COL_ACCENT) }),
        text("Syntax Guide").size(28).style(|_: &Theme| text::Style { color: Some(Color::WHITE) })
    ]
    .spacing(15)
    .align_y(iced::Alignment::Center);

    let content = column![
        title,
        
        // 1. FUNDAMENTALS
        help_card(
            "Organization", 
            crate::gui::icon::TAG,
            vec![
                entry("!1", "Priority High (1) to Low (9)", "!1, !5, !9"),
                entry("#tag", "Add category. Aliases expand automatically.", "#work, #home"),
                entry("~30m", "Estimated Duration (m/h/d/w).", "~30m, ~1.5h, ~2d"),
            ]
        ),

        // 2. TIMELINE
        help_card(
            "Timeline & Scheduling", 
            crate::gui::icon::CALENDAR,
            vec![
                entry("@date", "Due Date. Deadline for the task.", "@tomorrow, @2025-12-31"),
                entry("^date", "Start Date. Hides/sorts lower until date.", "^next week, ^2025-01-01"),
                entry("Offsets", "Add time from today.", "1d (1 day), 2w (2 weeks), 3mo (3 months), 4y (4 years)"),
                entry("Keywords", "Relative dates supported.", "today, tomorrow, next week, next year"),


            ]
        ),

        // 3. RECURRENCE
        help_card(
            "Recurrence", 
            crate::gui::icon::REPEAT,
            vec![
                entry("@daily", "Quick presets.", "@daily, @weekly, @monthly, @yearly"),
                entry("@every X", "Custom intervals.", "@every 3 days, @every 2 weeks"),
                entry("Note", "Recurrence calculates next date based on Start Date if present, else Due Date.", ""),
            ]
        ),

        // 4. POWER SEARCH
        help_card(
            "Search & Filtering", 
            crate::gui::icon::SHIELD, 
            vec![
                entry("text", "Matches summary or description.", "buy milk"),
                entry("#tag", "Filter by specific tag.", "#gardening"),
                entry("is:status", "Filter by state.", "is:done, is:ongoing, is:active"),
                entry("Operators", "Compare values (<, >, <=, >=).", "~<20m (less than 20 minutes), <!4 (urgent tasks)"),
                entry("  Dates", "Filter by timeframe.", "@<today (Overdue), ^>tomorrow"),
                entry("  Priority", "Filter by priority range.", "!<3 (High prio), !>=5"),
                entry("  Duration", "Filter by effort.", "~<15m (Quick tasks)"),
            ]
        ),

        // FOOTER
        container(
            column![
                button(
                    text("Close Help")
                        .size(16)
                        .width(Length::Fill)
                        .align_x(iced::alignment::Horizontal::Center)
                )
                .padding(12)
                .width(Length::Fixed(200.0))
                .style(iced::widget::button::primary)
                .on_press(Message::CloseHelp),
                
                text(format!("Cfait v{} \u{2022} GPL3 \u{2022} Trougnouf (Benoit Brummer)", env!("CARGO_PKG_VERSION")))
                     .size(12)
                     .style(|_: &Theme| text::Style { color: Some(COL_MUTED) })
            ]
            .spacing(15)
            .align_x(iced::Alignment::Center)
        )
        .width(Length::Fill)
        .center_x(Length::Fill)
        .padding(20)
    ]
    .spacing(20)
    .padding(20)
    .max_width(800);

    scrollable(
        container(content)
            .width(Length::Fill)
            .center_x(Length::Fill)
    )
    .height(Length::Fill)
    .into()
}

// --- HELPERS ---

struct HelpEntry {
    syntax: &'static str,
    desc: &'static str,
    example: &'static str,
}

fn entry(syntax: &'static str, desc: &'static str, example: &'static str) -> HelpEntry {
    HelpEntry { syntax, desc, example }
}

fn help_card(title: &'static str, icon_char: char, items: Vec<HelpEntry>) -> Element<'static, Message> {
    let header = row![
        crate::gui::icon::icon(icon_char).size(20).style(|_: &Theme| text::Style { color: Some(COL_ACCENT) }),
        text(title).size(18).style(|_: &Theme| text::Style { color: Some(COL_ACCENT) })
    ]
    .spacing(10)
    .align_y(iced::Alignment::Center);

    let mut rows = column![header, iced::widget::rule::horizontal(1).style(|_: &Theme| iced::widget::rule::Style { 
        color: Color::from_rgb(0.3, 0.3, 0.3),
        radius: 0.0.into(),
        fill_mode: iced::widget::rule::FillMode::Full,
        snap: true,
    })].spacing(12);

    for item in items {
        let syntax_pill = container(
            text::<Theme, iced::Renderer>(item.syntax)
                .size(14)
                .style(|_: &Theme| text::Style { color: Some(COL_SYNTAX) })
        )
        .padding([2, 6])
        .style(|_: &Theme| container::Style {
            background: Some(Color::from_rgba(1.0, 0.85, 0.4, 0.1).into()), 
            border: iced::Border {
                radius: 4.0.into(),
                ..Default::default()
            },
            ..Default::default()
        });

        let content = column![
            row![
                syntax_pill.width(Length::Fixed(110.0)),
                text::<Theme, iced::Renderer>(item.desc).size(14).width(Length::Fill).style(|_: &Theme| text::Style { color: Some(Color::WHITE) }),
            ].spacing(10).align_y(iced::Alignment::Center),
            
            if !item.example.is_empty() {
                Element::new(row![
                    Space::new().width(Length::Fixed(110.0)),
                    text::<Theme, iced::Renderer>(format!("e.g.: {}", item.example))
                        .size(12)
                        .style(|_: &Theme| text::Style { color: Some(COL_MUTED) })
                ])
            } else {
                Element::new(Space::new().height(0))
            }
        ].spacing(2);

        rows = rows.push(content);
    }

    container(rows)
        .padding(15)
        .style(|_: &Theme| container::Style {
            background: Some(COL_CARD_BG.into()),
            border: iced::Border {
                radius: 8.0.into(),
                width: 1.0,
                color: Color::from_rgb(0.25, 0.25, 0.28),
            },
            ..Default::default()
        })
        .width(Length::Fill)
        .into()
}