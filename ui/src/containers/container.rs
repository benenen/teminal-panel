use iced::widget;
use iced::{Element, Length};

pub struct Container<'a, Message> {
    content: Element<'a, Message>,
    width: Length,
    height: Length,
}

impl<'a, Message: 'a> Container<'a, Message> {
    pub fn new(content: Element<'a, Message>) -> Self {
        Self {
            content,
            width: Length::Shrink,
            height: Length::Shrink,
        }
    }

    pub fn width(mut self, width: Length) -> Self {
        self.width = width;
        self
    }

    pub fn height(mut self, height: Length) -> Self {
        self.height = height;
        self
    }

    pub fn into_element(self) -> Element<'a, Message> {
        widget::container(self.content)
            .width(self.width)
            .height(self.height)
            .into()
    }
}
