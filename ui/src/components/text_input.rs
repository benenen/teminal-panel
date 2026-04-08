use iced::widget;
use iced::{Element, Font};

pub struct TextInput<'a, Message> {
    placeholder: String,
    value: String,
    on_input: Option<Box<dyn Fn(String) -> Message + 'a>>,
    on_submit: Option<Message>,
}

impl<'a, Message: 'a + Clone> TextInput<'a, Message> {
    pub fn new(placeholder: impl Into<String>, value: &str) -> Self {
        Self {
            placeholder: placeholder.into(),
            value: value.to_string(),
            on_input: None,
            on_submit: None,
        }
    }

    pub fn on_input<F>(mut self, f: F) -> Self
    where
        F: Fn(String) -> Message + 'a,
    {
        self.on_input = Some(Box::new(f));
        self
    }

    pub fn on_submit(mut self, message: Message) -> Self {
        self.on_submit = Some(message);
        self
    }

    pub fn into_element(self) -> Element<'a, Message> {
        let mut input = widget::text_input(&self.placeholder, &self.value)
            .font(Font::MONOSPACE);

        if let Some(on_input) = self.on_input {
            input = input.on_input(on_input);
        }

        if let Some(on_submit) = self.on_submit {
            input = input.on_submit(on_submit);
        }

        input.into()
    }
}
