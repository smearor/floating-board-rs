# Senior Rust Developer - Clean Code Agent Guidelines

## Core Principles

### 1. Code Quality Standards

- **Readability First**: Code should be self-documenting and easy to understand
- **Simplicity**: Favor simple solutions over complex ones
- **Consistency**: Follow established patterns and conventions throughout the codebase
- **Testability**: Write code that is easy to test and maintain

### 2. Rust-Specific Clean Code Practices

#### Naming Conventions

- Use `snake_case` for functions and variables
- Use `PascalCase` for types, structs, and enums
- Use `SCREAMING_SNAKE_CASE` for constants
- Choose descriptive names that reveal intent
- Avoid abbreviations unless widely understood

#### Function Design

- Keep functions small and focused (single responsibility)
- Prefer explicit returns over implicit ones for clarity
- Use meaningful parameter names
- Limit function parameters to 3-4 when possible
- Use result types (`Result<T, E>`) for error handling

#### File and Module Organization

- **One struct/enum per file**: Each struct or enum should be in its own file to maintain clear separation of concerns
- **Logical module grouping**: Group related types into modules based on domain responsibility
- **Module structure**: Create a `mod.rs` in each module directory with module declarations and `pub use` re-exports for a clean API
- **Clear separation**: Ensure modules have distinct responsibilities - if types belong to different domains, separate them into different modules

### 3. Error Handling

- Use `Result<T, E>` for recoverable errors
- Use `Option<T>` for values that may be absent
- Create custom error types with `thiserror` or `anyhow`
- Handle errors at the appropriate level
- Avoid using `panic!()` in production code
- **Strict NewTypes and dedicated error types**: domain primitives are wrapped in newtype structs; every crate has its own `thiserror` error enum

### 4. Concurrency

- Use channels for message passing
- Use async/await for I/O-bound operations
- Avoid blocking operations in async contexts

## Testing Guidelines

- **Idiomatic Tests**: Use idiomatic Rust testing patterns
- **Inline Tests**: Keep tests in the same file as the source code
- **Comprehensive Coverage**: Test both success and error paths

## Project-Specific Requirements

### Rust Implementation Standards

- **Rust Edition 2024**: Use latest edition features
- **MSRV**: latest stable version minus 1 (currently **1.98.0**)
- **Panic-Free Code**: Avoid `unwrap()`, `expect()`, and panicking code
- **English Comments**: All source code comments in English
- **No Abbreviations**: Use descriptive variable names without abbreviations
- **Prefer Trait Implementations over Free Functions**: Use standard trait implementations (`FromStr`, `Display`, `Serialize`, `Deserialize`) where they fit

### Documentation Standards

- **Type Documentation**: All public enums and structs must have rustdoc comments describing their purpose
- **Enum Variants**: Each enum variant must be documented with a brief description
- **Struct Fields**: Each field in a struct must be documented
- **Language**: All documentation must be in English

### Import Organization

- **Individual Imports**: One import per line
- **No Star Imports**: Except for preludes (e.g., `gtk4::prelude::*`)
- **Alphabetical Ordering**: Sort all `use` statements alphabetically within each scope
- **Import Scope Order**: `crate::` first, then external crates, then `std::` last

### Formatting

- **Use `rustfmt`**: Run `cargo fmt` before committing (`.rustfmt.toml`: `max_width=160`, `comment_width=120`, `fn_call_width=110`, `imports_granularity="Item"`)
- **Single-line preference**: Prefer single-line style for macro invocations and function signatures when they remain readable
- **No trailing commas** in single-line constructs; use trailing commas only in multi-line blocks

### UI Text

- **Fluent i18n**: All user-facing strings go through `L10n` (`i18n/{en,de}/main.ftl`), never hardcoded literals

### Tools and Libraries

- `clippy` - Rust linter (`cargo clippy --workspace --all-targets -- -D warnings`)
- `rustfmt` - Code formatter
- `cargo-audit` - Security audit
- `thiserror` - Error handling
- `miette` - User-friendly error reporting
- `tracing` - Instrumentation (`debug!`, `info!`, ... with plain imports)
- `serde` - Serialization
- `fluent` - Localization

## Domain Notes

- The map anchor is the physical Eiffel tower tangible (`eifelturm-gross`
  profile); `rotation_rad` is correspondence-anchored to the profile pin
  frame (robofigure-rs >= bc109b8) - `heading_offset` calibrates map north.
- Corbeau secrecy: no unrevealed Corbeau position may enter public
  renderable state - the public view is `corbeau_public_view` only.
- Map dataset water rules (enforced in `GameMap::from_str`): the
  `bridge` station kind marks water crossings; every `batobus` pier
  must be a `bridge`; `taxi`/`bus` edges may only cross a
  `backdrop.water` polyline with a `bridge` endpoint; `metro`/
  `underground` edges are exempt (tunnels).

---

*This guide should be updated regularly to reflect best practices and team experience.*
