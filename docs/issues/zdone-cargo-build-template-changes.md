# Issue: cargo build doesn't detect template HTML changes

## Problem

When modifying Askama template files (`.html` files in `templates/`), `cargo build` does not detect the changes and will not recompile. This is because Cargo only tracks `.rs` files as dependencies by default, not template files.

## Workaround

After modifying a template file, touch the corresponding `.rs` handler file or run:

```bash
cargo clean && cargo build
```

Or touch any `.rs` file to force recompilation:

```bash
touch src/main.rs && cargo build
```

## Potential Solutions

1. **Use `build.rs`**: Add a build script that uses `rerun-if-changed` directives for template files.

2. **Use cargo-watch**: Run `cargo watch -x build` which can be configured to watch additional file patterns.

3. **Askama config**: Check if Askama has configuration options for this in `askama.toml`.

## References

- This is a known limitation of Askama's compile-time template system
- Related to how Cargo tracks file dependencies for incremental compilation
