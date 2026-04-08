mod agent;
mod app;
mod config;
mod terminal;

use app::App;

fn main() -> iced::Result {
    iced::application("teminal-panel", App::update, App::view)
        .theme(|_| App::theme())
        .subscription(App::subscription)
        .run_with(App::new)
}
