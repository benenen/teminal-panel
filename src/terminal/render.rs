use crate::app::Message;
use crate::terminal::model::TerminalModel;
use iced::widget::text;
use iced::Element;
use uuid::Uuid;

pub fn terminal_view<'a>(_terminal_id: Uuid, _model: &'a TerminalModel) -> Element<'a, Message> {
    text("Terminal renderer pending").into()
}
