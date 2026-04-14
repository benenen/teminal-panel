use iced::widget::container;
use iced::{Color, Element, Length};

pub struct ContextMenu<'a, Message> {
    content: Element<'a, Message>,
    width: Length,
}

impl<'a, Message: 'a> ContextMenu<'a, Message> {
    pub fn new(content: Element<'a, Message>) -> Self {
        Self {
            content,
            width: Length::Fixed(220.0),
        }
    }

    pub fn width(mut self, width: Length) -> Self {
        self.width = width;
        self
    }

    pub fn into_element(self) -> Element<'a, Message> {
        container(self.content)
            .width(self.width)
            .padding(8)
            .style(|_| {
                container::Style::default()
                    .background(Color::from_rgb(0.14, 0.14, 0.14))
                    .border(iced::Border {
                        color: Color::from_rgb(0.22, 0.22, 0.22),
                        width: 1.0,
                        radius: 10.into(),
                    })
                    .shadow(iced::Shadow {
                        color: Color::from_rgba(0.0, 0.0, 0.0, 0.35),
                        offset: iced::Vector::new(0.0, 8.0),
                        blur_radius: 24.0,
                    })
            })
            .into()
    }
}
