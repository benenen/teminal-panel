mod app;
mod config;
mod project;
mod ssh;
mod terminal;

use app::App;

fn main() -> iced::Result {
    iced::application(App::new, App::update, App::view)
        .theme(App::theme)
        .subscription(App::subscription)
        .font(iced_fonts::BOOTSTRAP_FONT_BYTES)
        .run()
}
