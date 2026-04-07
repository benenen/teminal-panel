mod agent;
mod app;
mod config;

use app::{App, Message};

fn main() -> iced::Result {
    iced::application("teminal-panel", App::update, App::view)
        .theme(|_| App::theme())
        .run_with(App::new)
}
