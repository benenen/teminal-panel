use termwiz::escape::parser::Parser;
use termwiz::surface::Surface;

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
        let _ = &mut self.parser;
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
