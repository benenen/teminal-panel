# teminal-ui

A reusable UI component library built with Iced.

## Components

### Button

```rust
use teminal_ui::components::Button;

Button::new("Click me")
    .on_press(Message::ButtonPressed)
    .into_element()
```

### TextInput

```rust
use teminal_ui::components::TextInput;

TextInput::new("Placeholder", &value)
    .on_input(Message::InputChanged)
    .on_submit(Message::Submit)
    .into_element()
```

### Modal

```rust
use teminal_ui::containers::Modal;

Modal::new(content)
    .with_title("Dialog Title")
    .into_element()
```

### Container

```rust
use teminal_ui::containers::Container;

Container::new(content)
    .width(Length::Fill)
    .height(Length::Fill)
    .into_element()
```

### Theme

```rust
use teminal_ui::Theme;

let theme = Theme::dark();
```

## Features

- Thin wrappers around Iced primitives
- Consistent styling and theming
- Reusable across projects
- Type-safe message handling
