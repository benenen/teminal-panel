mod app;
mod config;
mod git_window;
mod project;
mod ssh;
mod terminal;

use app::App;

fn main() -> iced::Result {
    iced::daemon(App::new, App::update, App::view_window)
        .title(App::title)
        .theme(App::theme)
        .subscription(App::subscription)
        .font(iced_fonts::BOOTSTRAP_FONT_BYTES)
        .run()
}
