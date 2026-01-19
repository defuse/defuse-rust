# Vim Highlighting Cache Intermittent Hangs

## Problem

The vim syntax highlighting cache appears to be failing intermittently during snapshot generation. The snapshot tool hangs on random pages that use vim highlighting.

## Symptoms

- Snapshot generation hangs indefinitely on pages with `ctx.hl_file` or `ctx.hl_string` calls
- The hanging page varies between runs (not consistently the same page)
- Suggests a race condition or locking issue in the cache mechanism

## Possible Causes

1. File locking issues in `/storage/vimhl/` cache directory
2. Race condition when multiple requests hit the same uncached content
3. Vim process not terminating properly in some cases
4. Cache file corruption causing infinite wait on lock

## Workaround

Clear the vim cache before running snapshots:
```bash
rm -rf /storage/vimhl/*
```

## Investigation Needed

- Review the locking mechanism in `src/libs/vim_highlight.rs`
- Check if vim processes are being properly terminated
- Consider adding timeouts to cache operations
