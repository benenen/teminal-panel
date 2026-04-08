use crate::app::{Message, TerminalViewport};
use crate::terminal::model::TerminalModel;
use iced::advanced::layout;
use iced::advanced::mouse;
use iced::advanced::renderer;
use iced::advanced::widget::{tree, Operation, Tree};
use iced::advanced::{Clipboard, Layout, Shell, Widget};
use iced::widget::canvas;
use iced::{alignment, Color, Element, Event, Font, Length, Pixels, Point, Rectangle, Renderer, Size, Theme};
use termwiz::cell::{CellAttributes, Intensity, Underline};
use termwiz::color::{ColorAttribute, SrgbaTuple};
use termwiz::surface::{CursorShape, CursorVisibility};
use uuid::Uuid;

pub const CELL_WIDTH: f32 = 8.0;
pub const CELL_HEIGHT: f32 = 16.0;
pub const FONT_SIZE: f32 = 14.0;

const DEFAULT_FOREGROUND: Color = Color::from_rgb(
    229.0 / 255.0,
    229.0 / 255.0,
    229.0 / 255.0,
);
const DEFAULT_BACKGROUND: Color = Color::from_rgb(
    30.0 / 255.0,
    30.0 / 255.0,
    30.0 / 255.0,
);

pub fn terminal_view<'a>(
    _terminal_id: Uuid,
    model: &'a TerminalModel,
    on_resize: impl Fn(TerminalViewport) -> Message + 'a,
) -> Element<'a, Message> {
    let canvas = canvas(TerminalCanvas { model })
        .width(Length::Fill)
        .height(Length::Fill);

    ViewportReporter::new(canvas, on_resize).into()
}

struct TerminalCanvas<'a> {
    model: &'a TerminalModel,
}

impl<Message> canvas::Program<Message, Theme, Renderer> for TerminalCanvas<'_> {
    type State = ();

    fn draw(
        &self,
        _state: &Self::State,
        renderer: &Renderer,
        _theme: &Theme,
        bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> Vec<canvas::Geometry<Renderer>> {
        let mut frame = canvas::Frame::new(renderer, bounds.size());
        let surface = self.model.surface();

        frame.fill_rectangle(Point::ORIGIN, bounds.size(), DEFAULT_BACKGROUND);

        for (row_index, line) in surface.screen_lines().iter().enumerate() {
            for cell in line.visible_cells() {
                let x = cell.cell_index() as f32 * CELL_WIDTH;
                let y = row_index as f32 * CELL_HEIGHT;
                let width = cell.width() as f32 * CELL_WIDTH;
                let attrs = cell.attrs();
                let (foreground, background) = resolved_colors(attrs);

                if background != DEFAULT_BACKGROUND {
                    frame.fill_rectangle(
                        Point::new(x, y),
                        Size::new(width, CELL_HEIGHT),
                        background,
                    );
                }

                if attrs.underline() != Underline::None {
                    frame.fill_rectangle(
                        Point::new(x, y + CELL_HEIGHT - 2.0),
                        Size::new(width, 1.5),
                        foreground,
                    );
                }

                if should_draw_text(cell.str()) {
                    frame.fill_text(canvas::Text {
                        content: cell.str().to_string(),
                        position: Point::new(x, y),
                        color: foreground,
                        size: Pixels(FONT_SIZE),
                        line_height: iced::widget::text::LineHeight::default(),
                        font: Font::MONOSPACE,
                        horizontal_alignment: alignment::Horizontal::Left,
                        vertical_alignment: alignment::Vertical::Top,
                        shaping: iced::widget::text::Shaping::Basic,
                    });
                }
            }
        }

        draw_cursor(&mut frame, surface);

        vec![frame.into_geometry()]
    }
}

fn draw_cursor(frame: &mut canvas::Frame<Renderer>, surface: &termwiz::surface::Surface) {
    if surface.cursor_visibility() != CursorVisibility::Visible {
        return;
    }

    let (x, y) = surface.cursor_position();
    let top_left = Point::new(x as f32 * CELL_WIDTH, y as f32 * CELL_HEIGHT);
    let cursor_color = DEFAULT_FOREGROUND;

    match surface.cursor_shape().unwrap_or(CursorShape::SteadyBlock) {
        CursorShape::BlinkingUnderline | CursorShape::SteadyUnderline => {
            frame.fill_rectangle(
                Point::new(top_left.x, top_left.y + CELL_HEIGHT - 2.0),
                Size::new(CELL_WIDTH, 2.0),
                cursor_color,
            );
        }
        CursorShape::BlinkingBar | CursorShape::SteadyBar => {
            frame.fill_rectangle(top_left, Size::new(2.0, CELL_HEIGHT), cursor_color);
        }
        _ => {
            frame.stroke_rectangle(
                top_left,
                Size::new(CELL_WIDTH, CELL_HEIGHT),
                canvas::Stroke::default().with_color(cursor_color).with_width(1.0),
            );
        }
    }
}

fn resolved_colors(attrs: &CellAttributes) -> (Color, Color) {
    let mut foreground = map_color(attrs.foreground(), false);
    let mut background = map_color(attrs.background(), true);

    if attrs.reverse() {
        std::mem::swap(&mut foreground, &mut background);
    }

    if attrs.intensity() == Intensity::Bold {
        foreground = brighten(foreground);
    }

    (foreground, background)
}

fn should_draw_text(text: &str) -> bool {
    !text.is_empty() && text != " "
}

fn brighten(color: Color) -> Color {
    Color {
        r: (color.r * 1.2).min(1.0),
        g: (color.g * 1.2).min(1.0),
        b: (color.b * 1.2).min(1.0),
        a: color.a,
    }
}

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
        ColorAttribute::TrueColorWithDefaultFallback(color)
        | ColorAttribute::TrueColorWithPaletteFallback(color, _) => srgb_to_iced(color),
    }
}

fn srgb_to_iced(color: SrgbaTuple) -> Color {
    Color::from_rgba(color.0, color.1, color.2, color.3)
}

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
}

#[derive(Default)]
struct ViewportReporterState {
    last_viewport: Option<TerminalViewport>,
}

impl<'a, Message> ViewportReporter<'a, Message> {
    fn new(
        content: impl Into<Element<'a, Message>>,
        on_resize: impl Fn(TerminalViewport) -> Message + 'a,
    ) -> Self {
        Self {
            content: content.into(),
            on_resize: Box::new(on_resize),
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
    use termwiz::color::ColorAttribute;

    #[test]
    fn color_attribute_mapping_preserves_basic_ansi_colors() {
        assert_eq!(
            map_color(ColorAttribute::PaletteIndex(1), false),
            Color::from_rgb8(205, 49, 49)
        );
    }
}
