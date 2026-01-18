# VimHighlight cache uses temporary file paths as cache keys

**Date:** 2026-01-18

**Severity:** Performance bug

## Problem

The vim_highlight module is using temporary file paths as cache keys:

```
2026-01-18T09:13:15.839310Z DEBUG defuse::libs::vim_highlight: Cache hit for "/tmp/.tmpBwVUzi"
```

Since each request creates a new temporary file with a unique name, the cache key will never match on subsequent requests for the same content. This defeats the purpose of caching entirely.

## Expected Behavior

The cache key should be based on:
- A hash of the file content, or
- The original source identifier (e.g., paste ID), or
- Some other stable identifier

## Current Behavior

Cache keys are based on ephemeral temp file paths like `/tmp/.tmpBwVUzi`, which are unique per request and provide no cache benefit.
