# Claude Code Instructions for defuse-rust

## Commit Discipline

**Make clear, atomic commits for every change.** After completing a logical unit of work:

1. Stage the relevant files
2. Write a clear commit message describing what changed and why
3. Commit before moving on to the next task

Commit messages should:
- Start with a verb (Add, Fix, Update, Remove, Refactor, etc.)
- Be concise but descriptive
- Reference the feature/component being changed

Examples:
- `Add checksums page with MD5/SHA hash support`
- `Fix template syntax for Askama compatibility`
- `Update base template to match original PHP layout`

## Project Structure

```
defuse-rust/
├── src/
│   ├── main.rs           # Application entry point, router setup
│   ├── context.rs        # Request context (IP, DNT, etc.)
│   └── pages/
│       ├── mod.rs        # Page module exports
│       ├── home.rs       # Home page handler
│       ├── about.rs      # About page handler
│       └── checksums.rs  # Checksums page handler
├── templates/
│   ├── base.html         # Master template (header, nav, footer)
│   └── pages/
│       ├── home.html
│       ├── about.html
│       └── checksums.html
├── static/
│   ├── main.css
│   ├── mainmenu.css
│   ├── vimhl.css
│   ├── print.css
│   ├── images/
│   └── js/
├── docs/                  # All documentation
└── CLAUDE.md             # This file (stays in root)
```

## Development Environment Setup

### Required Environment Variables

The following environment variables must be set (see `.env.example`):

- `STORAGE_PATH` - Path to storage directory containing:
  - `vimhl/` - VimHighlight cache
  - `extras/files/` - Large file downloads (force download)
  - `extras/files2/` - File downloads (viewable in browser)
  - `extras/mirrors/` - Mirrored content (force download)
  - `extras/upload/` - User uploads (force download)

For local development, set `STORAGE_PATH=../storage` to use the storage directory in the parent folder.

### Storage Directory Setup

```bash
# Create storage directories if needed
mkdir -p ../storage/vimhl
mkdir -p ../storage/extras/{files,files2,mirrors,upload}
```

The vimhl cache directory is used for caching vim-generated syntax highlighting output.

## Key Technical Details

- **Framework**: Axum web framework
- **Templating**: Askama (Jinja2-style, compile-time checked)
- **Database**: sqlx with MySQL (not yet integrated)
- **Original PHP site**: `../defuse.ca/` for reference
- **Vim**: Required for syntax highlighting (vim or gvim must be installed)

## Important: Matching Original Site

This is a rewrite of defuse.ca. The Rust version must:
- Produce identical HTML output where possible
- Support all original URLs (with redirects)
- Maintain database compatibility (same schema)
- Match cryptographic implementations exactly (for pastebin)

Reference the original PHP files in `../defuse.ca/src/` when implementing features.
