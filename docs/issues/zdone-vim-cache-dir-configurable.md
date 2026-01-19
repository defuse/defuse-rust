# Make vim highlighting cache directory configurable

## Current Behavior

The vim highlighting cache directory is hardcoded in `src/libs/vim_highlight.rs`:

```rust
const CACHE_DIR: &str = "/storage/vimhl";
```

## Desired Behavior

The cache directory should be configurable via:
- Environment variable (e.g., `VIMHL_CACHE_DIR`)
- Or configuration file

This would allow:
- Different cache locations for dev vs prod
- Easier local development without needing `/storage/vimhl`
- Docker/container deployments with custom paths

## Files to modify

- `src/libs/vim_highlight.rs` - Read cache dir from env/config instead of const
