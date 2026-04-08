use iced::widget;
use iced::{Element, Length};

pub struct Button<Message> {
    label: String,
    on_press: Option<Message>,
    width: Length,
}

impl<Message: Clone + 'static> Button<Message> {
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            on_press: None,
            width: Length::Shrink,
        }
    }

    pub fn on_press(mut self, message: Message) -> Self {
        self.on_press = Some(message);
        self
    }

    pub fn width(mut self, width: Length) -> Self {
        self.width = width;
        self
    }

    pub fn into_element(self) -> Element<'static, Message> {
        let mut btn = widget::button(widget::text(self.label)).width(self.width);

        if let Some(message) = self.on_press {
            btn = btn.on_press(message);
        }

        btn.into()
    }
}
