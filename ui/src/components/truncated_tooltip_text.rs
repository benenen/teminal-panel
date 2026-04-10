use iced::widget::{container, text, tooltip};
use iced::{Color, Element, Length};

pub struct TruncatedTooltipText {
    value: String,
    max_chars: usize,
    size: u32,
    width: Length,
}

impl TruncatedTooltipText {
    pub fn new(value: impl Into<String>) -> Self {
        Self {
            value: value.into(),
            max_chars: 24,
            size: 12,
            width: Length::Shrink,
        }
    }

    pub fn max_chars(mut self, max_chars: usize) -> Self {
        self.max_chars = max_chars;
        self
    }

    pub fn size(mut self, size: u32) -> Self {
        self.size = size;
        self
    }

    pub fn width(mut self, width: Length) -> Self {
        self.width = width;
        self
    }

    pub fn into_element<Message: Clone + 'static>(self) -> Element<'static, Message> {
        let truncated = truncate(&self.value, self.max_chars);

        let label = text(truncated).size(self.size).width(self.width);

        if self.value.chars().count() <= self.max_chars {
            label.into()
        } else {
            tooltip(
                container(label).width(self.width),
                container(text(self.value).size(self.size))
                    .padding([6, 8])
                    .style(|_| {
                        container::Style::default()
                            .background(Color::from_rgb8(45, 45, 45))
                            .border(iced::Border {
                                color: Color::from_rgb8(100, 100, 100),
                                width: 1.0,
                                radius: 6.0.into(),
                            })
                    }),
                tooltip::Position::FollowCursor,
            )
            .into()
        }
    }
}

fn truncate(value: &str, max_chars: usize) -> String {
    let len = value.chars().count();

    if len <= max_chars {
        return value.to_string();
    }

    if max_chars <= 1 {
        return "…".to_string();
    }

    let visible: String = value.chars().take(max_chars - 1).collect();
    format!("{visible}…")
}
