mod agent;
mod app;
mod config;

use app::App;

fn main() -> iced::Result {
    iced::application("teminal-panel", App::update, App::view)
        .theme(App::theme)
        .run_with(App::new)
}
