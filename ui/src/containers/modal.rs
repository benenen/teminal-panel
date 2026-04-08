use iced::widget::{column, container, text};
use iced::{Color, Element, Length};

pub struct Modal<'a, Message> {
    content: Element<'a, Message>,
    title: Option<String>,
    width: Length,
}

impl<'a, Message: 'a> Modal<'a, Message> {
    pub fn new(content: Element<'a, Message>) -> Self {
        Self {
            content,
            title: None,
            width: Length::Fixed(400.0),
        }
    }

    pub fn with_title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    pub fn width(mut self, width: Length) -> Self {
        self.width = width;
        self
    }

    pub fn into_element(self) -> Element<'a, Message> {
        let mut modal_content = column![];

        if let Some(title) = self.title {
            modal_content = modal_content.push(text(title).size(20));
        }

        modal_content = modal_content.push(self.content);

        let modal = container(modal_content)
            .width(self.width)
            .style(|_| {
                container::Style::default()
                    .background(Color::from_rgb8(45, 45, 45))
                    .border(iced::Border {
                        color: Color::from_rgb8(100, 100, 100),
                        width: 1.0,
                        radius: 8.0.into(),
                    })
            })
            .padding(20);

        container(modal)
            .width(Length::Fill)
            .height(Length::Fill)
            .center_x(Length::Fill)
            .center_y(Length::Fill)
            .into()
    }
}
