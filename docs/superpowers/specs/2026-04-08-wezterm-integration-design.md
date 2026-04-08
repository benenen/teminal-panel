# wezterm 库集成设计

**Date:** 2026-04-08
**Status:** Design Phase
**Scope:** Phase 2 - Replace termwiz with wezterm libraries

## Overview

Replace termwiz library with wezterm-term and wezterm-escape-parser for better terminal emulation and ANSI parsing. This is a complete replacement of the terminal backend while maintaining 100% feature parity and all existing tests.

## Goals

- Replace termwiz with wezterm libraries (wezterm-term + wezterm-escape-parser)
- Update terminal module to use wezterm types and APIs
- Maintain 100% feature parity with current implementation
- All existing 16 tests pass without modification
- Improve terminal emulation quality and ANSI parsing accuracy

## Architecture

### Dependency Changes

**Current (termwiz):**
```toml
termwiz = "0.23.3"
```

**New (wezterm):**
```toml
wezterm-term = "0.1"
wezterm-escape-parser = "0.1"
```

**Rationale:**
- `wezterm-term` - Production-grade terminal emulator (replaces termwiz::surface::Surface)
- `wezterm-escape-parser` - Robust ANSI escape sequence parser (replaces termwiz escape parsing)
- `portable-pty` - Unchanged (PTY management is independent)

### Core Data Structure Changes

**Current (termwiz):**
```rust
pub struct TerminalModel {
    surface: Surface,  // termwiz::surface::Surface
    dirty: bool,
}
```

**New (wezterm):**
```rust
pub struct TerminalModel {
    terminal: Terminal,  // wezterm_term::Terminal
    parser: Parser,      // wezterm_escape_parser::Parser
    dirty: bool,
}
```

**Key Changes:**
- `Surface` → `Terminal` (wezterm's terminal emulator)
- Add `Parser` for ANSI escape sequence processing
- Internal API completely changed, but public interface remains compatible

### Module Structure

**terminal/model.rs:**
- Replace all `termwiz::*` imports with `wezterm_term::*` and `wezterm_escape_parser::*`
- Update `TerminalModel::new()` to initialize wezterm Terminal
- Update `process_output()` to use wezterm Parser for ANSI sequence parsing
- Update `surface()` method to return wezterm Terminal data
- Update `resize()` method to call wezterm Terminal's resize API
- Update `is_dirty()` and `mark_clean()` methods (unchanged logic)

**terminal/render.rs:**
- Replace `use termwiz::cell::*` with wezterm cell types
- Update `render_cell()` to read attributes from wezterm cells
- Update color handling (wezterm color representation may differ)
- Update cursor position and visibility handling
- Update cell width and attribute processing

**terminal/subscription.rs:**
- No changes needed (only handles PTY output, independent of terminal backend)

**terminal/pty.rs:**
- No changes needed (PTY management is independent)

## Implementation Strategy

### Phase 2a: Update Dependencies and Core Types

1. Update `teminal-panel/Cargo.toml`:
   - Remove `termwiz = "0.23.3"`
   - Add `wezterm-term = "0.1"`
   - Add `wezterm-escape-parser = "0.1"`

2. Update `terminal/model.rs`:
   - Replace imports
   - Update `TerminalModel` struct
   - Implement `new()` with wezterm Terminal initialization
   - Update `process_output()` with wezterm Parser

3. Verify compilation (will have errors in render.rs, that's expected)

### Phase 2b: Update Rendering

1. Update `terminal/render.rs`:
   - Replace termwiz cell imports
   - Update `render_cell()` function
   - Update color handling
   - Update cursor and attribute processing

2. Verify compilation and all tests pass

### Phase 2c: Testing and Verification

1. Run all 16 app tests
2. Run clippy for warnings
3. Manual testing: Run application and verify terminal display
4. Verify colors, styles, and cursor position

## Type Mappings

### Cell Attributes

**termwiz:**
```rust
use termwiz::cell::{CellAttributes, Intensity, Underline};
use termwiz::color::ColorAttribute;
```

**wezterm:**
```rust
use wezterm_term::cell::{CellAttributes, Intensity, Underline};
use wezterm_term::color::ColorAttribute;
```

### Surface/Terminal

**termwiz:**
```rust
use termwiz::surface::{Surface, Change, Position};
```

**wezterm:**
```rust
use wezterm_term::Terminal;
// Position and Change concepts handled differently
```

### ANSI Parsing

**termwiz:**
```rust
use termwiz::escape::Action;
// Parsing integrated into Surface
```

**wezterm:**
```rust
use wezterm_escape_parser::Parser;
// Explicit parser for escape sequences
```

## Testing Strategy

### Existing Tests

- All 16 app tests should pass without modification
- Tests only verify public API behavior, not internal implementation
- No test code changes required

### New Verification

- Verify ANSI escape sequences parse correctly
- Verify colors and styles render correctly
- Verify cursor position is accurate
- Manual testing: Run application and verify terminal display

### Success Criteria

✅ All 16 existing tests pass
✅ Application compiles with no errors
✅ Terminal displays content correctly
✅ Colors and styles render correctly
✅ Cursor position is accurate
✅ No clippy warnings related to changes

## Risks & Mitigations

| Risk | Mitigation |
|------|-----------|
| wezterm API differences | Study wezterm documentation, implement adapter if needed |
| Color handling differences | Test color rendering thoroughly, adjust if needed |
| Performance regression | Profile before/after, optimize if needed |
| Breaking changes in wezterm | Pin specific version, document compatibility |

## Out of Scope

- Updating UI crate (no changes needed)
- Modifying teminal-panel app logic (only terminal backend changes)
- Adding new terminal features (focus on replacement only)
- Performance optimization (beyond what wezterm provides)

## Success Criteria

✅ wezterm libraries integrated successfully
✅ All 16 existing tests pass
✅ Terminal functionality identical to before
✅ Code compiles with no errors
✅ No clippy warnings related to changes
✅ Manual testing confirms correct display

## Next Steps (Future)

After Phase 2 is complete:
1. Consider adding wezterm-specific features (if desired)
2. Performance profiling and optimization
3. Extended terminal feature support
4. Cross-platform testing
