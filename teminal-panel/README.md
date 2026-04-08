# teminal-panel

A GUI application for managing multiple project terminals.

## Building

```bash
cargo build -p teminal-panel
```

## Running

```bash
cargo run -p teminal-panel
```

## Testing

```bash
cargo test -p teminal-panel
```

## Architecture

- **ui crate** - Reusable UI components
- **teminal-panel crate** - Application using ui components
- **terminal module** - Terminal emulation and PTY management
- **config module** - Configuration persistence
- **project module** - Project management

See CLAUDE.md for detailed architecture documentation.
