// File: ./src/gui/mod.rs
pub mod async_ops;
pub mod icon;
pub mod message;
pub mod state;
pub mod subscription;
pub mod update;
pub mod view;

use crate::config::Config;
use crate::gui::message::Message;
use crate::gui::state::GuiApp;
use iced::{Element, Subscription, Task, Theme, font, window};

pub fn run() -> iced::Result {
    // Initialize the Tokio runtime managed in async_ops
    async_ops::init_runtime();

    iced::application(
        "Cfait | 🗹 Take control of your TODO list",
        GuiApp::update,
        GuiApp::view,
    )
    .subscription(GuiApp::subscription)
    .theme(GuiApp::theme)
    .window(window::Settings {
        decorations: false, // <--- Disable OS Top Bar
        platform_specific: window::settings::PlatformSpecific {
            #[cfg(target_os = "linux")]
            application_id: String::from("cfait"),

            ..Default::default()
        },
        ..Default::default()
    })
    .run_with(GuiApp::new)
}

impl GuiApp {
    fn new() -> (Self, Task<Message>) {
        (
            Self::default(),
            Task::batch(vec![
                // Load config
                Task::perform(
                    async { Config::load().map_err(|e| e.to_string()) },
                    Message::ConfigLoaded,
                ),
                // Load Font Bytes
                font::load(icon::FONT_BYTES).map(|_| Message::FontLoaded(Ok(()))),
            ]),
        )
    }

    fn view(&self) -> Element<'_, Message> {
        view::root_view(self)
    }

    fn theme(&self) -> Theme {
        Theme::Dark
    }

    fn subscription(&self) -> Subscription<Message> {
        subscription::subscription(self)
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        update::update(self, message)
    }
}
