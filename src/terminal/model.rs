use termwiz::cell::AttributeChange;
use termwiz::color::ColorAttribute;
use termwiz::escape::{
    csi::{CSI, Cursor, Edit, EraseInDisplay, EraseInLine, Sgr},
    parser::Parser,
    Action, ControlCode,
};
use termwiz::surface::{Change, Position, Surface};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TerminalSize {
    pub cols: u16,
    pub rows: u16,
}

pub struct TerminalModel {
    parser: Parser,
    surface: Surface,
    size: TerminalSize,
    dirty: bool,
}

impl TerminalModel {
    pub fn new(cols: u16, rows: u16) -> Self {
        Self {
            parser: Parser::new(),
            surface: Surface::new(cols as usize, rows as usize),
            size: TerminalSize { cols, rows },
            dirty: false,
        }
    }

    pub fn advance_bytes(&mut self, _bytes: &[u8]) {
        let mut actions = Vec::new();
        self.parser.parse(_bytes, |action| actions.push(action));

        for action in actions {
            apply_action(&mut self.surface, action);
        }

        self.dirty = true;
    }

    pub fn resize(&mut self, size: TerminalSize) {
        if self.size == size {
            return;
        }

        self.surface.resize(size.cols as usize, size.rows as usize);
        self.size = size;
        self.dirty = true;
    }

    pub fn size(&self) -> TerminalSize {
        self.size
    }

    pub fn surface(&self) -> &Surface {
        &self.surface
    }

    pub fn is_dirty(&self) -> bool {
        self.dirty
    }

    pub fn mark_clean(&mut self) {
        self.dirty = false;
    }
}

fn apply_action(surface: &mut Surface, action: Action) {
    match action {
        Action::Print(c) => {
            let _ = surface.add_change(Change::Text(c.to_string()));
        }
        Action::PrintString(text) => {
            let _ = surface.add_change(Change::Text(text));
        }
        Action::Control(ControlCode::CarriageReturn) => {
            let (_, row) = surface.cursor_position();
            let _ = surface.add_change(Change::CursorPosition {
                x: Position::Absolute(0),
                y: Position::Absolute(row),
            });
        }
        Action::Control(ControlCode::LineFeed) => {
            let _ = surface.add_change(Change::Text("\n".into()));
        }
        Action::CSI(csi) => apply_csi(surface, csi),
        _ => {}
    }
}

fn apply_csi(surface: &mut Surface, csi: CSI) {
    match csi {
        CSI::Sgr(sgr) => apply_sgr(surface, sgr),
        CSI::Cursor(cursor) => apply_cursor(surface, cursor),
        CSI::Edit(edit) => apply_edit(surface, edit),
        _ => {}
    }
}

fn apply_sgr(surface: &mut Surface, sgr: Sgr) {
    match sgr {
        Sgr::Reset => {
            let _ = surface.add_change(Change::AllAttributes(Default::default()));
        }
        Sgr::Intensity(intensity) => {
            let _ = surface.add_change(Change::Attribute(AttributeChange::Intensity(intensity)));
        }
        Sgr::Underline(underline) => {
            let _ = surface.add_change(Change::Attribute(AttributeChange::Underline(underline)));
        }
        Sgr::Italic(enabled) => {
            let _ = surface.add_change(Change::Attribute(AttributeChange::Italic(enabled)));
        }
        Sgr::Inverse(enabled) => {
            let _ = surface.add_change(Change::Attribute(AttributeChange::Reverse(enabled)));
        }
        Sgr::Foreground(color) => {
            let _ = surface.add_change(Change::Attribute(AttributeChange::Foreground(color.into())));
        }
        Sgr::Background(color) => {
            let _ = surface.add_change(Change::Attribute(AttributeChange::Background(color.into())));
        }
        _ => {}
    }
}

fn apply_cursor(surface: &mut Surface, cursor: Cursor) {
    let (x, y) = surface.cursor_position();

    match cursor {
        Cursor::Left(n) => {
            let _ = surface.add_change(Change::CursorPosition {
                x: Position::Absolute(x.saturating_sub(n as usize)),
                y: Position::Absolute(y),
            });
        }
        Cursor::Right(n) => {
            let _ = surface.add_change(Change::CursorPosition {
                x: Position::Absolute(x.saturating_add(n as usize)),
                y: Position::Absolute(y),
            });
        }
        Cursor::Up(n) => {
            let _ = surface.add_change(Change::CursorPosition {
                x: Position::Absolute(x),
                y: Position::Absolute(y.saturating_sub(n as usize)),
            });
        }
        Cursor::Down(n) => {
            let _ = surface.add_change(Change::CursorPosition {
                x: Position::Absolute(x),
                y: Position::Absolute(y.saturating_add(n as usize)),
            });
        }
        Cursor::CharacterAbsolute(col) | Cursor::CharacterPositionAbsolute(col) => {
            let _ = surface.add_change(Change::CursorPosition {
                x: Position::Absolute(col.as_zero_based() as usize),
                y: Position::Absolute(y),
            });
        }
        Cursor::LinePositionAbsolute(line) => {
            let _ = surface.add_change(Change::CursorPosition {
                x: Position::Absolute(x),
                y: Position::Absolute(line.saturating_sub(1) as usize),
            });
        }
        Cursor::Position { line, col } | Cursor::CharacterAndLinePosition { line, col } => {
            let _ = surface.add_change(Change::CursorPosition {
                x: Position::Absolute(col.as_zero_based() as usize),
                y: Position::Absolute(line.as_zero_based() as usize),
            });
        }
        Cursor::NextLine(n) => {
            let _ = surface.add_change(Change::CursorPosition {
                x: Position::Absolute(0),
                y: Position::Absolute(y.saturating_add(n as usize)),
            });
        }
        Cursor::PrecedingLine(n) => {
            let _ = surface.add_change(Change::CursorPosition {
                x: Position::Absolute(0),
                y: Position::Absolute(y.saturating_sub(n as usize)),
            });
        }
        _ => {}
    }
}

fn apply_edit(surface: &mut Surface, edit: Edit) {
    match edit {
        Edit::EraseInDisplay(EraseInDisplay::EraseDisplay) => {
            let _ = surface.add_change(Change::ClearScreen(ColorAttribute::Default));
        }
        Edit::EraseInDisplay(EraseInDisplay::EraseToEndOfDisplay) => {
            let _ = surface.add_change(Change::ClearToEndOfScreen(ColorAttribute::Default));
        }
        Edit::EraseInLine(EraseInLine::EraseToEndOfLine) => {
            let _ = surface.add_change(Change::ClearToEndOfLine(ColorAttribute::Default));
        }
        Edit::EraseInLine(EraseInLine::EraseLine) => {
            let (_, row) = surface.cursor_position();
            let _ = surface.add_change(Change::CursorPosition {
                x: Position::Absolute(0),
                y: Position::Absolute(row),
            });
            let _ = surface.add_change(Change::ClearToEndOfLine(ColorAttribute::Default));
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::{TerminalModel, TerminalSize};
    use termwiz::color::ColorAttribute;

    #[test]
    fn advance_plain_text_updates_surface() {
        let mut model = TerminalModel::new(5, 3);

        model.advance_bytes(b"abc");

        assert!(model.surface.screen_chars_to_string().starts_with("abc"));
    }

    #[test]
    fn advance_sgr_color_sets_cell_attributes() {
        let mut model = TerminalModel::new(5, 3);

        model.advance_bytes(b"\x1b[31mR");

        let cells = model.surface.screen_cells();
        let first = &cells[0][0];
        assert_eq!(first.str(), "R");
        assert_ne!(first.attrs().foreground(), ColorAttribute::Default);
    }

    #[test]
    fn clear_display_erases_prior_text() {
        let mut model = TerminalModel::new(5, 3);

        model.advance_bytes(b"abc");
        assert!(model.surface.screen_chars_to_string().contains("abc"));

        model.advance_bytes(b"\x1b[2J");

        assert!(!model.surface.screen_chars_to_string().contains("abc"));
    }

    #[test]
    fn resize_updates_surface_dimensions() {
        let mut model = TerminalModel::new(80, 24);

        model.resize(TerminalSize { cols: 100, rows: 40 });

        assert_eq!(model.size(), TerminalSize { cols: 100, rows: 40 });
        assert_eq!(model.surface.dimensions(), (100, 40));
    }
}
