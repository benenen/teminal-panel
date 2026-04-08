# Plan 2: termwiz ANSI Terminal Rendering Design

## Goal

Integrate a real terminal emulator into the existing local PTY flow so the right-hand panel renders ANSI/VT output as a terminal screen instead of appending raw text. This phase also adds PTY resize synchronization so terminal programs can react to panel size changes.

## Scope

Included in this phase:

- Full ANSI/VT output handling through `termwiz`
- Screen rendering based on terminal cells, colors, and cursor state
- PTY resize synchronization driven by terminal viewport changes
- Preservation of the existing local PTY lifecycle and output subscription pipeline
- Test coverage for parser/model behavior and resize coordination

Explicitly excluded from this phase:

- Full special-key encoding such as arrows, function keys, Alt combos, and mouse protocol
- Text selection, copy/paste UI, or search
- SSH transport
- Precise double-width or complex grapheme layout beyond what the fixed-grid renderer can safely support in this phase

## Current Baseline

Plan 1 already provides:

- Local PTY spawn and lifecycle management through `portable-pty`
- A bounded subscription channel from PTY output into the `iced` application update loop
- Basic terminal input by writing submitted text plus newline to PTY stdin
- Agent persistence, terminal opening, and cleanup behavior

The current terminal view stores a raw `String` and renders it inside a scrollable text widget. ANSI control sequences are not interpreted, cursor movement is lost, and terminal programs do not receive updated row/column information when the view size changes.

## Architecture

The new architecture keeps PTY transport and terminal emulation separate:

1. `src/terminal/pty.rs` remains responsible for spawning the shell, reading and writing PTY bytes, and adds an explicit resize API.
2. A new terminal model layer owns a `termwiz::escape::parser::Parser`, a `termwiz::surface::Surface`, and the metadata needed to render and resize them safely.
3. A new terminal renderer layer converts the surface state into `iced` drawing primitives using a fixed cell grid.
4. `src/app.rs` continues to orchestrate application state, but PTY bytes are forwarded into the terminal model instead of appended into a raw output buffer.

Data flow:

`PTY bytes -> termwiz parser -> local action-to-surface adapter -> termwiz surface -> iced renderer`

This keeps terminal semantics inside the emulator while the UI remains a thin rendering and layout layer.

## Terminal Model

Each open terminal session will be split conceptually into transport state and emulation state.

### Transport state

The transport side continues to store:

- PTY writer for stdin
- PTY lifecycle handle for shutdown
- Terminal identifier and associated agent identifier

It also gains:

- The last applied terminal size in rows/cols
- A resize entry point that can update the underlying PTY without recreating the session

### Emulation state

The emulation side will own:

- A `termwiz` parser
- A `termwiz` surface
- The current logical terminal size in rows/cols
- Cached viewport metrics used by the renderer
- Enough dirtiness metadata to know when a redraw is needed

The raw `String output` field will be removed from `TerminalState`. The model becomes the source of truth for visible content, cursor location, and surface-backed screen state.

Because `wezterm-term` is not published as a normal crates.io dependency, this phase uses the published `termwiz` crate directly and adds a narrow local adapter that maps parsed ANSI actions onto a `Surface`.

## Rendering Strategy

Rendering will move from `text()` widgets to a dedicated terminal view that paints a fixed grid.

### Grid model

The renderer assumes:

- Monospace font
- Fixed cell width and line height constants for the first iteration
- One visible rectangular viewport per terminal

For every visible cell, the renderer reads:

- Display text
- Foreground color
- Background color
- Text attributes that are practical to support in this phase, such as bold and underline

### Drawing order

The terminal should render in three passes:

1. Background rectangles per visible cell or contiguous runs
2. Glyphs for visible cell contents
3. Cursor overlay based on terminal cursor position and visibility

This ordering keeps cursor painting separate from text output and makes later cursor styles easier to extend.

### Styling rules

Phase 2 will support the practical subset needed for common shell usage:

- Default colors
- ANSI indexed colors
- Truecolor values exposed by the emulator
- Bold
- Underline
- Reverse video when present through the terminal cell attributes

Unsupported or awkward attributes can degrade to the closest readable representation rather than blocking the feature.

## Resize Synchronization

The terminal viewport will compute rows and columns from the available pixel size and the configured cell metrics.

When the computed dimensions change:

1. Update the terminal model size
2. Resize the emulator state if required by the chosen API
3. Call the PTY resize API with the same rows/cols
4. Trigger a redraw

Resize failures should not destroy the terminal session. The app should keep the previous working size, report the error locally, and continue rendering the last valid screen.

## Error Handling

Error handling stays conservative:

- PTY read failure or child exit preserves the last rendered frame and marks the session as closed
- Terminal parsing errors do not panic the application; byte advancement is delegated to the emulator library and wrapped defensively
- PTY resize failure does not tear down the session
- Renderer-side unsupported attributes degrade gracefully to a simpler visible style

## Files and Responsibilities

Planned file decomposition:

- Modify `Cargo.toml`
- Add the `termwiz` dependency and any small supporting crates needed for rendering state
- Modify `src/terminal/mod.rs`
  - Replace raw text-oriented terminal state with model-oriented session state
- Modify `src/terminal/pty.rs`
  - Add PTY resize support
- Create `src/terminal/model.rs`
  - Own the `termwiz` parser, `Surface`, and ANSI action application logic
- Create `src/terminal/render.rs`
  - Convert terminal screen state into an `iced`-renderable grid or canvas program
- Modify `src/app.rs`
  - Route PTY output into the terminal model
  - Track viewport-derived rows/cols updates
  - Replace raw text rendering with the terminal renderer

Depending on `iced` API constraints discovered during implementation, a small `src/terminal/theme.rs` or `src/terminal/metrics.rs` helper module may also be justified, but that should remain optional.

## Testing Strategy

Testing will be layered.

### Model tests

Feed ANSI byte sequences directly into the terminal model and assert:

- Colored text updates screen cells with the expected foreground/background values
- Cursor movement produces the expected cell placements
- Clear-screen sequences remove prior content
- Wrapped lines and scrolling update the visible screen as expected

### App integration tests

Assert that:

- `Message::PtyOutput` advances terminal state rather than appending raw text
- Resize-related messages or callbacks result in PTY resize calls only when dimensions actually change
- Existing lifecycle behavior on agent removal and app drop remains intact

### Manual verification

Under Xvfb or a normal desktop session, validate:

- `printf` with ANSI colors
- `clear`
- Reflow and full-screen program behavior after resizing
- Common shell commands such as `ls --color=always`

## Risks

### `termwiz` integration details

`termwiz` provides the parser and screen primitives we need, but not a drop-in emulator object matching the original design wording. To contain that risk, the terminal model must isolate parser-to-surface adaptation behind a narrow local interface.

### `iced` rendering tradeoffs

`iced` does not ship a terminal widget. The renderer may need to use `canvas` or a custom widget path. The implementation should start with a fixed-grid renderer rather than trying to solve perfect text shaping in this phase.

### Resize feedback loops

Viewport changes can occur frequently during layout. Resize calls must be deduplicated by rows/cols to avoid spamming the PTY with redundant resize operations.

## Success Criteria

Plan 2 is complete when:

- ANSI output is rendered as a terminal screen rather than plain appended text
- Cursor movement, clear-screen behavior, and color output are visibly correct for common shell programs
- Terminal rows/cols update when the terminal viewport size changes
- Existing local PTY open/close behavior remains intact
- Automated tests cover model advancement and resize coordination
