// File: ./src/gui/subscription.rs
use crate::gui::message::Message;
use crate::gui::state::{AppState, GuiApp};
use iced::{Subscription, event, keyboard, mouse, window};

pub fn subscription(app: &GuiApp) -> Subscription<Message> {
    use iced::keyboard::key;

    let mut subs = Vec::new();

    if matches!(app.state, AppState::Onboarding | AppState::Settings) {
        subs.push(keyboard::on_key_press(|k, modifiers| {
            if k == key::Key::Named(key::Named::Tab) {
                Some(Message::TabPressed(modifiers.shift()))
            } else {
                None
            }
        }));
    }

    // Subscribe to mouse events if resizing
    if app.resize_direction.is_some() {
        subs.push(event::listen_with(|evt, _status, _window_id| match evt {
            iced::Event::Mouse(mouse::Event::CursorMoved { position }) => {
                Some(Message::ResizeUpdate(position))
            }
            iced::Event::Mouse(mouse::Event::ButtonReleased(_)) => Some(Message::ResizeEnd),
            _ => None,
        }));
    }

    // Track window metrics (Size and Position)
    subs.push(event::listen_with(|evt, _status, _window_id| match evt {
        iced::Event::Window(window::Event::Resized(size)) => Some(Message::WindowResized(size)),
        iced::Event::Window(window::Event::Moved(point)) => Some(Message::WindowMoved(point)),
        _ => None,
    }));

    Subscription::batch(subs)
}
