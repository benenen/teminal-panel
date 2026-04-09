use std::io::Write;
use std::sync::Arc;
use termwiz::surface::CursorVisibility;
use wezterm_cell::UnicodeVersion;
use wezterm_escape_parser::parser::Parser;
use wezterm_term::Terminal;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TerminalSize {
    pub cols: u16,
    pub rows: u16,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TerminalCluster {
    pub col: usize,
    pub text: String,
    pub width: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TerminalCursor {
    pub row: usize,
    pub col: usize,
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

    fn unicode_version(&self) -> UnicodeVersion {
        // SAFETY: UnicodeVersion is a struct in wezterm_cell.
        // We use zeroed() as a placeholder since the trait requires returning this type.
        // This is a temporary workaround until wezterm_term exposes a public constructor.
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

    pub fn visible_rows(&self) -> Vec<Vec<TerminalCluster>> {
        let size = self.terminal.get_size();
        let screen = self.terminal.screen();
        let start = screen.phys_row(0);

        screen
            .lines_in_phys_range(start..start + size.rows)
            .into_iter()
            .map(|line| {
                line.cluster(None)
                    .into_iter()
                    .map(|cluster| TerminalCluster {
                        col: cluster.first_cell_idx,
                        text: cluster.text,
                        width: cluster.width.max(1),
                    })
                    .collect()
            })
            .collect()
    }

    pub fn visible_text_rows(&self) -> Vec<String> {
        self.visible_rows()
            .into_iter()
            .map(|row| {
                let mut text = String::new();
                let mut next_col = 0;

                for cell in row {
                    while next_col < cell.col {
                        text.push(' ');
                        next_col += 1;
                    }

                    text.push_str(&cell.text);
                    next_col += cell.width;
                }

                text
            })
            .collect()
    }

    pub fn cursor(&self) -> Option<TerminalCursor> {
        let cursor = self.terminal.cursor_pos();
        if cursor.visibility != CursorVisibility::Visible {
            return None;
        }

        let size = self.terminal.get_size();
        if cursor.y < 0 || cursor.y as usize >= size.rows || cursor.x >= size.cols {
            return None;
        }

        Some(TerminalCursor {
            row: cursor.y as usize,
            col: cursor.x,
        })
    }

    pub fn cols(&self) -> usize {
        self.terminal.get_size().cols
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
    use super::{TerminalCluster, TerminalModel, TerminalSize};

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

    #[test]
    fn process_output_populates_visible_text_rows() {
        let mut model = TerminalModel::new(80, 24);
        model.process_output(b"hello");

        let joined = model.visible_text_rows().join("\n");
        assert!(joined.contains("hello"), "visible rows: {joined:?}");
    }

    #[test]
    fn visible_rows_preserve_cluster_columns() {
        let mut model = TerminalModel::new(80, 24);
        model.process_output(b"abc");

        assert_eq!(
            model.visible_rows()[0],
            vec![TerminalCluster {
                col: 0,
                text: "abc".into(),
                width: 3
            }]
        );
    }

    #[test]
    fn visible_rows_preserve_double_width_clusters() {
        let mut model = TerminalModel::new(80, 24);
        model.process_output("中".as_bytes());

        assert_eq!(
            model.visible_rows()[0],
            vec![TerminalCluster {
                col: 0,
                text: "中".into(),
                width: 2
            }]
        );
    }

    #[test]
    fn cursor_respects_visibility() {
        let model = TerminalModel::new(80, 24);

        assert_eq!(
            model.cursor(),
            Some(super::TerminalCursor { row: 0, col: 0 })
        );
    }

    #[test]
    fn visible_text_rows_expand_cluster_widths() {
        let mut model = TerminalModel::new(80, 24);
        model.process_output("中a".as_bytes());

        let first = &model.visible_text_rows()[0];
        assert!(first.starts_with("中a"), "first row: {first:?}");
        let total_width: usize = model.visible_rows()[0]
            .iter()
            .map(|cluster| cluster.width)
            .sum();
        assert_eq!(total_width, 3);
    }
}
