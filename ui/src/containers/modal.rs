use iced::widget::{button, column, container, row, text};
use iced::{Color, Element, Length};
use iced_fonts::bootstrap;

pub struct Modal<'a, Message> {
    content: Element<'a, Message>,
    title: Option<String>,
    width: Length,
    on_close: Option<Message>,
}

impl<'a, Message: Clone + 'a> Modal<'a, Message> {
    pub fn new(content: Element<'a, Message>) -> Self {
        Self {
            content,
            title: None,
            width: Length::Fixed(400.0),
            on_close: None,
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

    pub fn on_close(mut self, message: Message) -> Self {
        self.on_close = Some(message);
        self
    }

    pub fn into_element(self) -> Element<'a, Message> {
        let mut modal_content = column![];

        if self.title.is_some() || self.on_close.is_some() {
            let title = self.title.unwrap_or_default();
            let mut header = row![text(title).size(22).width(Length::Fill)]
                .align_y(iced::alignment::Vertical::Center)
                .height(Length::Fixed(32.0));

            if let Some(message) = self.on_close {
                header = header.push(
                    button(bootstrap::x_lg().size(12))
                        .on_press(message)
                        .padding([4, 6])
                        .style(button::text),
                );
            }

            modal_content = modal_content.push(header);
        }

        modal_content = modal_content.push(self.content);

        let modal = container(modal_content.spacing(20))
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
            .padding(24);

        container(modal)
            .width(Length::Fill)
            .height(Length::Fill)
            .style(|_| container::Style::default().background(Color::from_rgba(0.0, 0.0, 0.0, 0.5)))
            .center_x(Length::Fill)
            .center_y(Length::Fill)
            .into()
    }
}
