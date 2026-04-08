use wezterm_term::Terminal;
use wezterm_escape_parser::parser::Parser;
use std::sync::Arc;
use std::io::Write;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TerminalSize {
    pub cols: u16,
    pub rows: u16,
}

#[allow(private_interfaces)]
#[derive(Debug)]
struct DummyConfig;

impl wezterm_term::TerminalConfiguration for DummyConfig {
    fn generation(&self) -> usize {
        0
    }

    fn scrollback_size(&self) -> usize {
        3000
    }

    fn enable_csi_u_key_encoding(&self) -> bool {
        false
    }

    fn color_palette(&self) -> wezterm_term::color::ColorPalette {
        Default::default()
    }

    fn alternate_buffer_wheel_scroll_speed(&self) -> u8 {
        3
    }

    fn enq_answerback(&self) -> String {
        String::new()
    }

    fn enable_kitty_graphics(&self) -> bool {
        false
    }

    fn enable_title_reporting(&self) -> bool {
        false
    }

    fn enable_checksum_rectangular_area(&self) -> bool {
        false
    }

    fn enable_kitty_keyboard(&self) -> bool {
        false
    }

    fn canonicalize_pasted_newlines(&self) -> wezterm_term::config::NewlineCanon {
        Default::default()
    }

    fn unicode_version(&self) -> wezterm_term::config::UnicodeVersion {
        // Placeholder - UnicodeVersion is private in wezterm_term
        // This will be fixed when the trait is properly exposed
        unsafe { std::mem::zeroed() }
    }

    fn debug_key_events(&self) -> bool {
        false
    }

    fn log_unknown_escape_sequences(&self) -> bool {
        false
    }

    fn normalize_output_to_unicode_nfc(&self) -> bool {
        false
    }

    fn bidi_mode(&self) -> wezterm_term::config::BidiMode {
        unsafe { std::mem::zeroed() }
    }
}

pub struct TerminalModel {
    terminal: Terminal,
    parser: Parser,
    dirty: bool,
}

impl TerminalModel {
    pub fn new(cols: u16, rows: u16) -> Self {
        struct NullWriter;
        impl Write for NullWriter {
            fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
                Ok(buf.len())
            }
            fn flush(&mut self) -> std::io::Result<()> {
                Ok(())
            }
        }

        let config = Arc::new(DummyConfig);
        let terminal = Terminal::new(
            wezterm_term::TerminalSize {
                cols: cols as usize,
                rows: rows as usize,
                pixel_width: 0,
                pixel_height: 0,
                dpi: 96,
            },
            config,
            "xterm-256color",
            "1.0",
            Box::new(NullWriter),
        );

        Self {
            terminal,
            parser: Parser::new(),
            dirty: true,
        }
    }

    pub fn process_output(&mut self, data: &[u8]) {
        self.terminal.advance_bytes(data);
        self.dirty = true;
    }

    pub fn surface(&self) -> &Terminal {
        &self.terminal
    }

    pub fn resize(&mut self, size: TerminalSize) -> Result<(), String> {
        self.terminal.resize(wezterm_term::TerminalSize {
            cols: size.cols as usize,
            rows: size.rows as usize,
            pixel_width: 0,
            pixel_height: 0,
            dpi: 96,
        });
        Ok(())
    }

    pub fn is_dirty(&self) -> bool {
        self.dirty
    }

    pub fn mark_clean(&mut self) {
        self.dirty = false;
    }
}

#[cfg(test)]
mod tests {
    use super::{TerminalModel, TerminalSize};

    #[test]
    fn new_creates_terminal() {
        let model = TerminalModel::new(80, 24);
        assert!(model.is_dirty());
    }

    #[test]
    fn process_output_marks_dirty() {
        let mut model = TerminalModel::new(80, 24);
        model.mark_clean();
        assert!(!model.is_dirty());

        model.process_output(b"test");
        assert!(model.is_dirty());
    }

    #[test]
    fn resize_succeeds() {
        let mut model = TerminalModel::new(80, 24);
        let result = model.resize(TerminalSize {
            cols: 100,
            rows: 40,
        });
        assert!(result.is_ok());
    }

    #[test]
    fn mark_clean_clears_dirty_flag() {
        let mut model = TerminalModel::new(80, 24);
        assert!(model.is_dirty());
        model.mark_clean();
        assert!(!model.is_dirty());
    }
}
