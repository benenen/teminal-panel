use iced::Color;

pub struct Theme {
    pub primary_color: Color,
    pub background_color: Color,
    pub text_color: Color,
    pub border_color: Color,
    pub modal_background: Color,
}

impl Theme {
    pub fn dark() -> Self {
        Self {
            primary_color: Color::from_rgb8(100, 100, 100),
            background_color: Color::from_rgb8(30, 30, 30),
            text_color: Color::from_rgb8(229, 229, 229),
            border_color: Color::from_rgb8(100, 100, 100),
            modal_background: Color::from_rgb8(45, 45, 45),
        }
    }

    pub fn light() -> Self {
        Self {
            primary_color: Color::from_rgb8(200, 200, 200),
            background_color: Color::from_rgb8(240, 240, 240),
            text_color: Color::from_rgb8(30, 30, 30),
            border_color: Color::from_rgb8(150, 150, 150),
            modal_background: Color::from_rgb8(220, 220, 220),
        }
    }
}
