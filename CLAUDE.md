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

## Documentation Location

**All project documentation goes in `docs/`**

When creating new markdown files for:
- Requirements specifications
- Design decisions
- Implementation notes
- TODO/tracking files

Place them in the `docs/` folder, not the project root.

Current docs:
- `docs/DESIGN_DECISIONS.md` - Architectural choices and rationale
- `docs/TODO.md` - Project progress tracker
- `docs/URL_ROUTING_REQUIREMENTS.md` - URL routing system specification

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

Before running the project, ensure these directories exist:

```bash
# VimHighlight cache directory (for syntax highlighting)
sudo mkdir -p /storage/vimhl
sudo chown $USER:$USER /storage/vimhl
```

The cache directory is used by both the PHP original and Rust rewrite for caching vim-generated syntax highlighting output.

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
