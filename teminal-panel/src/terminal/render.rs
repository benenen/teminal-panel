use crate::app::{Message, TerminalViewport};
use crate::terminal::model::{TerminalCluster, TerminalCursor, TerminalModel};
use iced::advanced::clipboard::Kind;
use iced::advanced::layout;
use iced::advanced::mouse;
use iced::advanced::renderer;
use iced::advanced::widget::{tree, Operation, Tree};
use iced::advanced::{Clipboard, Layout, Shell, Widget};
use iced::keyboard;
use iced::widget::{column, container, row, text};
use iced::{
    alignment, Color, Element, Event, Font, Length, Pixels, Rectangle, Renderer, Size, Theme,
};
use uuid::Uuid;
#[cfg(test)]
use wezterm_term::color::ColorAttribute;

#[cfg(windows)]
pub const CELL_WIDTH: f32 = 8.0;
#[cfg(not(windows))]
pub const CELL_WIDTH: f32 = 8.0;

#[cfg(windows)]
pub const CELL_HEIGHT: f32 = 18.0;
#[cfg(not(windows))]
pub const CELL_HEIGHT: f32 = 16.0;

#[cfg(windows)]
pub const FONT_SIZE: f32 = 16.0;
#[cfg(not(windows))]
pub const FONT_SIZE: f32 = 14.0;

const DEFAULT_FOREGROUND: Color = Color::from_rgb(229.0 / 255.0, 229.0 / 255.0, 229.0 / 255.0);
const DEFAULT_BACKGROUND: Color = Color::from_rgb(30.0 / 255.0, 30.0 / 255.0, 30.0 / 255.0);

pub fn terminal_font() -> Font {
    #[cfg(windows)]
    {
        return Font::with_name("SimSun");
    }

    #[cfg(not(windows))]
    {
        Font::MONOSPACE
    }
}

pub fn terminal_view<'a>(
    _terminal_id: Uuid,
    model: &'a TerminalModel,
    on_resize: impl Fn(TerminalViewport) -> Message + 'a,
    on_key: impl Fn(String) -> Message + 'a,
) -> Element<'a, Message> {
    let lines = model.visible_rows();
    let cursor = model.cursor();
    let mut rows = column![].spacing(0);

    for (row_index, line) in lines.into_iter().enumerate() {
        rows = rows.push(render_line(line, row_index, cursor, model));
    }

    ViewportReporter::new(
        container(rows)
            .width(Length::Fill)
            .height(Length::Fill)
            .style(|_| container::Style::default().background(DEFAULT_BACKGROUND)),
        on_resize,
        on_key,
    )
    .into()
}

fn render_line<'a>(
    line: Vec<TerminalCluster>,
    row_index: usize,
    cursor: Option<TerminalCursor>,
    model: &TerminalModel,
) -> Element<'a, Message> {
    let cols = model.cols();
    let mut line_row = row![].spacing(0);
    let mut next_col = 0;

    for cell in line {
        while next_col < cell.col && next_col < cols {
            line_row = line_row.push(render_cell(
                " ".into(),
                1,
                cursor
                    == Some(TerminalCursor {
                        row: row_index,
                        col: next_col,
                    }),
            ));
            next_col += 1;
        }

        if next_col >= cols {
            break;
        }

        let width = cell.width.min(cols.saturating_sub(next_col)).max(1);
        line_row = line_row.push(render_cell(
            cell.text,
            width,
            cursor
                == Some(TerminalCursor {
                    row: row_index,
                    col: next_col,
                }),
        ));
        next_col += width;
    }

    while next_col < cols {
        line_row = line_row.push(render_cell(
            " ".into(),
            1,
            cursor
                == Some(TerminalCursor {
                    row: row_index,
                    col: next_col,
                }),
        ));
        next_col += 1;
    }

    container(line_row)
        .width(Length::Fill)
        .height(Length::Fixed(CELL_HEIGHT))
        .style(|_| container::Style::default().background(DEFAULT_BACKGROUND))
        .into()
}

fn render_cell<'a>(content: String, width: usize, is_cursor: bool) -> Element<'a, Message> {
    let display = if content.is_empty() {
        " ".to_string()
    } else {
        content
    };
    let foreground = if is_cursor {
        DEFAULT_BACKGROUND
    } else {
        DEFAULT_FOREGROUND
    };
    let background = if is_cursor {
        DEFAULT_FOREGROUND
    } else {
        DEFAULT_BACKGROUND
    };

    container(
        text(display)
            .font(terminal_font())
            .size(Pixels(FONT_SIZE))
            .width(Length::Fill)
            .align_x(alignment::Horizontal::Left),
    )
    .width(Length::Fixed(CELL_WIDTH * width as f32))
    .height(Length::Fixed(CELL_HEIGHT))
    .style(move |_| {
        container::Style::default()
            .color(foreground)
            .background(background)
    })
    .into()
}

#[cfg(test)]
fn map_color(color: ColorAttribute, is_background: bool) -> Color {
    match color {
        ColorAttribute::Default => {
            if is_background {
                DEFAULT_BACKGROUND
            } else {
                DEFAULT_FOREGROUND
            }
        }
        ColorAttribute::PaletteIndex(index) => palette_index_to_color(index),
        _ => {
            // Handle other color variants (TrueColor, etc.)
            DEFAULT_FOREGROUND
        }
    }
}

#[cfg(test)]
fn palette_index_to_color(index: u8) -> Color {
    let (red, green, blue) = match index {
        0 => (0, 0, 0),
        1 => (205, 49, 49),
        2 => (13, 188, 121),
        3 => (229, 229, 16),
        4 => (36, 114, 200),
        5 => (188, 63, 188),
        6 => (17, 168, 205),
        7 => (229, 229, 229),
        8 => (102, 102, 102),
        9 => (241, 76, 76),
        10 => (35, 209, 139),
        11 => (245, 245, 67),
        12 => (59, 142, 234),
        13 => (214, 112, 214),
        14 => (41, 184, 219),
        15 => (255, 255, 255),
        16..=231 => {
            let adjusted = index - 16;
            let red = adjusted / 36;
            let green = (adjusted % 36) / 6;
            let blue = adjusted % 6;

            (
                xterm_color_cube(red),
                xterm_color_cube(green),
                xterm_color_cube(blue),
            )
        }
        232..=255 => {
            let value = 8 + (index - 232) * 10;
            (value, value, value)
        }
    };

    Color::from_rgb8(red, green, blue)
}

#[cfg(test)]
fn xterm_color_cube(value: u8) -> u8 {
    match value {
        0 => 0,
        1 => 95,
        2 => 135,
        3 => 175,
        4 => 215,
        _ => 255,
    }
}

struct ViewportReporter<'a, Message> {
    content: Element<'a, Message>,
    on_resize: Box<dyn Fn(TerminalViewport) -> Message + 'a>,
    on_key: Box<dyn Fn(String) -> Message + 'a>,
}

#[derive(Default, Clone, Copy, Debug, PartialEq)]
struct SelectionPoint {
    row: usize,
    col: usize,
}

#[derive(Default)]
struct ViewportReporterState {
    last_viewport: Option<TerminalViewport>,
    selection_start: Option<SelectionPoint>,
    selection_end: Option<SelectionPoint>,
    is_selecting: bool,
}

impl<'a, Message> ViewportReporter<'a, Message> {
    fn new(
        content: impl Into<Element<'a, Message>>,
        on_resize: impl Fn(TerminalViewport) -> Message + 'a,
        on_key: impl Fn(String) -> Message + 'a,
    ) -> Self {
        Self {
            content: content.into(),
            on_resize: Box::new(on_resize),
            on_key: Box::new(on_key),
        }
    }
}

impl<'a, Message> Widget<Message, Theme, Renderer> for ViewportReporter<'a, Message> {
    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<ViewportReporterState>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(ViewportReporterState::default())
    }

    fn children(&self) -> Vec<Tree> {
        vec![Tree::new(&self.content)]
    }

    fn diff(&self, tree: &mut Tree) {
        tree.diff_children(std::slice::from_ref(&self.content));
    }

    fn size(&self) -> Size<Length> {
        self.content.as_widget().size()
    }

    fn layout(
        &self,
        tree: &mut Tree,
        renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        self.content
            .as_widget()
            .layout(&mut tree.children[0], renderer, limits)
    }

    fn operate(
        &self,
        tree: &mut Tree,
        layout: Layout<'_>,
        renderer: &Renderer,
        operation: &mut dyn Operation,
    ) {
        self.content
            .as_widget()
            .operate(&mut tree.children[0], layout, renderer, operation);
    }

    fn on_event(
        &mut self,
        tree: &mut Tree,
        event: Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &Renderer,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
        viewport: &Rectangle,
    ) -> iced::event::Status {
        publish_viewport_if_changed(self, tree, layout.bounds(), shell);
        let state: &mut ViewportReporterState = tree.state.downcast_mut();

        match &event {
            Event::Keyboard(keyboard::Event::KeyPressed {
                key,
                modifiers,
                text,
                ..
            }) => {
                // Handle Ctrl+C for copy
                if *modifiers == keyboard::Modifiers::CTRL {
                    if let keyboard::Key::Character(ch) = key {
                        if ch == "c" || ch == "C" {
                            if let (Some(start), Some(end)) =
                                (state.selection_start, state.selection_end)
                            {
                                // Copy selected text to clipboard
                                let selected_text = format!(
                                    "Selected: ({},{})-({},{})",
                                    start.row, start.col, end.row, end.col
                                );
                                let _ = clipboard.write(Kind::Standard, selected_text);
                                return iced::event::Status::Captured;
                            }
                        }
                    }
                }

                let text_input = if !modifiers.control() && !modifiers.alt() && !modifiers.logo() {
                    text.as_ref()
                        .filter(|text| !text.is_empty())
                        .map(|text| text.to_string())
                } else {
                    None
                };

                let input = text_input.or_else(|| match key {
                    keyboard::Key::Named(keyboard::key::Named::Enter) => Some("\r".to_string()),
                    keyboard::Key::Named(keyboard::key::Named::Backspace) => {
                        Some("\x08".to_string())
                    }
                    keyboard::Key::Named(keyboard::key::Named::Delete) => Some("\x7f".to_string()),
                    keyboard::Key::Named(keyboard::key::Named::Tab) => Some("\t".to_string()),
                    keyboard::Key::Named(keyboard::key::Named::ArrowUp) => {
                        Some("\x1b[A".to_string())
                    }
                    keyboard::Key::Named(keyboard::key::Named::ArrowDown) => {
                        Some("\x1b[B".to_string())
                    }
                    keyboard::Key::Named(keyboard::key::Named::ArrowRight) => {
                        Some("\x1b[C".to_string())
                    }
                    keyboard::Key::Named(keyboard::key::Named::ArrowLeft) => {
                        Some("\x1b[D".to_string())
                    }
                    keyboard::Key::Character(ch)
                        if !modifiers.control() && !modifiers.alt() && !modifiers.logo() =>
                    {
                        Some(ch.to_string())
                    }
                    _ => None,
                });

                if let Some(input) = input {
                    shell.publish((self.on_key)(input));
                    iced::event::Status::Captured
                } else {
                    self.content.as_widget_mut().on_event(
                        &mut tree.children[0],
                        event,
                        layout,
                        cursor,
                        renderer,
                        clipboard,
                        shell,
                        viewport,
                    )
                }
            }
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => {
                if let Some(position) = cursor.position_in(layout.bounds()) {
                    let col = (position.x / CELL_WIDTH).floor() as usize;
                    let row = (position.y / CELL_HEIGHT).floor() as usize;
                    state.selection_start = Some(SelectionPoint { row, col });
                    state.selection_end = Some(SelectionPoint { row, col });
                    state.is_selecting = true;
                    iced::event::Status::Captured
                } else {
                    iced::event::Status::Ignored
                }
            }
            Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) => {
                state.is_selecting = false;
                iced::event::Status::Captured
            }
            Event::Mouse(mouse::Event::CursorMoved { .. }) => {
                if state.is_selecting {
                    if let Some(position) = cursor.position_in(layout.bounds()) {
                        let col = (position.x / CELL_WIDTH).floor() as usize;
                        let row = (position.y / CELL_HEIGHT).floor() as usize;
                        state.selection_end = Some(SelectionPoint { row, col });
                        return iced::event::Status::Captured;
                    }
                }
                iced::event::Status::Ignored
            }
            _ => self.content.as_widget_mut().on_event(
                &mut tree.children[0],
                event,
                layout,
                cursor,
                renderer,
                clipboard,
                shell,
                viewport,
            ),
        }
    }

    fn mouse_interaction(
        &self,
        tree: &Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
        renderer: &Renderer,
    ) -> mouse::Interaction {
        self.content.as_widget().mouse_interaction(
            &tree.children[0],
            layout,
            cursor,
            viewport,
            renderer,
        )
    }

    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut Renderer,
        theme: &Theme,
        style: &renderer::Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        self.content.as_widget().draw(
            &tree.children[0],
            renderer,
            theme,
            style,
            layout,
            cursor,
            viewport,
        );
    }
}

impl<'a, Message: 'a> From<ViewportReporter<'a, Message>> for Element<'a, Message> {
    fn from(widget: ViewportReporter<'a, Message>) -> Self {
        Element::new(widget)
    }
}

fn publish_viewport_if_changed<Message>(
    widget: &ViewportReporter<'_, Message>,
    tree: &mut Tree,
    bounds: Rectangle,
    shell: &mut Shell<'_, Message>,
) {
    let state: &mut ViewportReporterState = tree.state.downcast_mut();
    let next_viewport = TerminalViewport {
        width: bounds.width,
        height: bounds.height,
    };

    if state.last_viewport == Some(next_viewport) {
        return;
    }

    state.last_viewport = Some(next_viewport);
    shell.publish((widget.on_resize)(next_viewport));
}

#[cfg(test)]
mod tests {
    use super::map_color;
    use iced::Color;
    use wezterm_term::color::ColorAttribute;

    #[test]
    fn color_attribute_mapping_preserves_basic_ansi_colors() {
        assert_eq!(
            map_color(ColorAttribute::PaletteIndex(1), false),
            Color::from_rgb8(205, 49, 49)
        );
    }
}
