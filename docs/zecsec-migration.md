# ZecSec.com Migration to defuse-rust

## Goal

Serve the content currently hosted at `zecsec.com` (a Hugo site at `../defuse.github.io/`) from the defuse-rust Axum server, with:

- `zecsec.com` redirecting to `defuse.ca` (path-aware, see R1)
- ZecSec blog posts served at `defuse.ca/zecsec/{slug}.htm`
- Audit report PDFs and images hosted from defuse-rust's static files
- Old `zecsec.com` URLs continuing to work via Caddy redirect rules

## Current ZecSec Site Structure

Hugo site in `../defuse.github.io/` with:

### Pages (3)
- `content/pages/overview.md` — massive Zcash ecosystem security overview (~680 lines)
- `content/pages/audits.md` — short audit listing (mostly "forthcoming")
- `content/pages/contact.md` — contact info with PGP key

### Posts (20)
| File | Title | Date | Notes |
|------|-------|------|-------|
| `my-first-post.md` | Hello, World! | 2022-10-13 | |
| `october-update.md` | October Update: Ywallet audited | 2022-10-28 | |
| `scalable-private-money-needs-scalable-private-messaging.md` | Scalable Private Money Needs Scalable Anonymous Messaging | 2022-11-15 | |
| `security-audit-process.md` | Security Audit Process | 2022-12-29 | |
| `2022-q4-transparency-report.md` | ZecSec's Q4 2022 Transparency Report | 2023-01-27 | Hugo `{{<table>}}` shortcode |
| `ywallet-audit-published.md` | YWallet Audit Results Published | 2023-01-03 | |
| `zecsec-roadmap-for-2023.md` | ZecSec Roadmap for 2023 | 2023-01-03 | |
| `making-zcash-light-wallets-faster-and-more-private.md` | Making Zcash Light Wallets Faster and More Private | 2023-03-02 | KaTeX (`katex: true`) |
| `risk-analysis-of-intel-sgx-and-other-tees.md` | Risk Analysis of Intel's SGX and Other TEEs | 2023-03-10 | |
| `threat-model-for-zcash-hardware-wallets.md` | A Simple Threat Model for Zcash Shielded Hardware Wallets | 2023-04-15 | |
| `2023-q1-transparency-report.md` | ZecSec's Q1 2023 Transparency Report | 2023-04-18 | Hugo `{{<table>}}` shortcode |
| `security-engineering.md` | Security Engineering: Learning from Safety-Critical Disciplines | 2023-07-30 | Hugo `{{<youtube>}}` shortcode |
| `milk-sad.md` | If you used libbitcoin-explorer (bx) to generate your seed phrase, rotate it ASAP! | 2023-08-11 | |
| `free2z-security-audit.md` | Free2Z Security Audit Results | 2023-09-14 | |
| `zcash-51-percent-attack.md` | Mitigating 51% Attack Risk on the Zcash Network | 2023-09-21 | |
| `security-audit-of-hanhs-ledger-app.md` | Results of Auditing Hanh's Shielded Zcash Ledger App | 2023-09-21 | |
| `zecwallet-lite-cli-security-audit.md` | Security Audit of zecwallet-lite-cli | 2023-09-23 | |
| `future-of-zcash-ecosystem-security.md` | The Future of Zcash Ecosystem Security | 2023-09-24 | |
| `zgo-security-audit.md` | ZGo Security Audit Results | 2024-01-05 | |

### Static Assets
- `static/audits/` — 5 PDF audit reports
- `static/images/bug-chart.png` — used in `future-of-zcash-ecosystem-security.md`
- `static/favicon-32.png`, `static/zcash-white-on-black-full.png`

### Hugo Shortcodes Used
- `{{< ref "some-post.md" >}}` — internal cross-references (many posts/pages)
- `{{< youtube fgkFrxiB14g >}}` — YouTube embed (1 post)
- `{{< toc >}}` — table of contents (1 page: overview)
- `{{<table "...">}}...{{</table>}}` — styled tables (2 posts: transparency reports)

### Hugo Frontmatter Fields
- `title`, `date`, `author`, `draft`, `slug` (on overview page), `katex` (on 1 post)

## Plan

### 1. Caddy Configuration

Add a site block for `zecsec.com, www.zecsec.com`:
- `zecsec.com/posts/{slug}/` → 301 redirect to `https://defuse.ca/zecsec/{slug}.htm`
- `zecsec.com/posts/{slug}` (no trailing slash) → same
- `zecsec.com/*` (everything else: root, `/overview/`, `/contact/`, etc.) → 301 redirect to `https://defuse.ca/zecsec.htm`

DNS A/AAAA records for `zecsec.com` and `www.zecsec.com` must point to the same IP as `defuse.ca`. Caddy handles TLS cert provisioning automatically via ACME.

### 2. Pre-process Markdown Files

Before adding files to the project, pre-process the Hugo markdown to remove Hugo-specific syntax. Copy the results into `static/markdown/zecsec/`. This is a one-time manual/scripted step:

- **Strip YAML frontmatter** (`---` delimited blocks at the top) — metadata (title, date) is manually copied into the page registry, no need to keep or parse frontmatter
- **Resolve `{{< ref "file.md" >}}`** → replace with `/zecsec/{slug}.htm` URLs using a static filename-to-slug mapping
- **Replace `{{< youtube ID >}}`** → `<div class="yt-container"><iframe src="https://www.youtube.com/embed/ID" ...></iframe></div>`
- **Replace `{{< toc >}}`** → remove (or generate TOC HTML if we want it for the overview page)
- **Replace `{{<table "classes">}}...{{</table>}}`** → strip the shortcode wrapper (let the markdown table render normally)
- **Update image/asset paths** → `/zecsec/images/...`, `/zecsec/audits/...`

Commit the pre-processed markdown files. The originals remain in `../defuse.github.io/` for reference.

### 3. Static Assets

- Copy `static/audits/*.pdf` → `static/zecsec/audits/`
- Copy `static/images/bug-chart.png` → `static/zecsec/images/`
- No need for favicon or zcash logo (using defuse.ca's design)

### 4. Markdown Rendering (`markdown_page!` macro)

Create a `markdown_page!` macro similar to `simple_page!` that:
- Takes a struct name and path to a pre-processed markdown file (frontmatter already stripped)
- Uses `include_str!` to embed the markdown at compile time
- Renders to HTML via comrak (new `render_post` function in `libs/markdown.rs`)
- `render_post` differs from `render_readme`: does NOT strip `# heading`, does NOT demote headings, does NOT strip badge lines
- Passes rendered HTML into a generic zecsec post template

Each markdown file must start with a `# Title` heading — this becomes the `<h1>` on the page. (The registry `title` field is only used for HTML `<title>` metadata, not rendered as a visible heading.)

The date comes from the registry and is rendered by the base template. The template (`templates/pages/zecsec/post.html`) just outputs the rendered markdown HTML.

### 5. Add `date` Field to Page Registry

Add a `date: Option<&'static str>` field to `PageInfo` (in `src/registry/mod.rs`). This is mandatory in the struct but `Option` so existing pages that set their date ad-hoc in template HTML can use `None` for now.

- Add `date` to `PageInfo` struct
- Update `page!` macro: default to `None`, add an arm or field syntax for specifying a date
- Update `alias!` macro to propagate `date` from the source page
- Update `NOT_FOUND_PAGE_INFO` with `date: None`
- The base template (`templates/base.html`) renders the date automatically when `Some`: `<div class="pagedate">{{ date }}</div>`, replacing the per-template `pagedate` divs
- ZecSec posts all have their dates manually copied from the Hugo frontmatter into the registry (e.g. `date: Some("January 27, 2023")`)
- Existing pages with ad-hoc dates in their template HTML (like the Gödel theorem page's `<div class="pagedate">February 9, 2024</div>`) can be migrated later by setting `date` in the registry and removing the HTML — but this is out of scope for the initial migration

This enables a future "list posts by date" feature across the whole site.

### 6. Page Registry Entries

Register all 20 posts in `src/registry/pages.rs`:
- Slug: `zecsec/{filename-stem}` (e.g. `zecsec/milk-sad`)
- Title from the frontmatter
- The KaTeX post (`making-zcash-light-wallets-faster-and-more-private`) gets `features: Features { math: true, banner: None }`
- All others get default features
- Category: a new `zecsec` upvote category (or reuse `defuse_research`)

The 3 "pages" (overview, audits, contact) are folded into the landing page — no separate registry entries needed for them.

### 7. Page Handlers

Each post gets a handler module under `src/pages/zecsec/`. With the `markdown_page!` macro, each file is a one-liner:
```rust
crate::markdown_page!(MilkSadPage, "zecsec/milk-sad.md");
```

Add `pub mod zecsec;` to `src/pages/mod.rs` with submodules for each post.

### 8. Landing Page (`zecsec.htm`)

A `simple_page!` with a hand-written HTML template at `templates/pages/zecsec/index.html` that consolidates all zecsec "page" content (overview, audits, contact) plus the post listing. This becomes the single comprehensive ZecSec page. The user will clean it up later.

The overview/audits/contact markdown content is manually converted to HTML for this template (not rendered through comrak). Hugo shortcodes and internal links are resolved to final HTML/URLs.

Include:
- Brief intro about the ZecSec project
- Chronological list of all posts (title, date, link)
- Full content from `overview.md` (the ecosystem security overview with all project listings, audit history, notable bugs, academic papers, etc.)
- Full content from `audits.md` (audit report listing with links to PDFs)
- Full content from `contact.md` (email, Discord, PGP key)

This means the 3 "pages" (overview, audits, contact) do NOT need their own separate page entries — they're all folded into the landing page.

### 9. Navigation

**WARNING: There are TWO copies of the navbar in `templates/base.html` — the desktop nav and the mobile/narrow-screen nav. Both must be kept in sync for every menu change.**

Rather than a single "ZecSec" link, distribute zecsec posts into the relevant existing nav sections. Use best judgement for categorization; get user confirmation before finalizing. Tentative sorting:

**Audits section:**
- YWallet Audit Results Published
- Free2Z Security Audit Results
- Results of Auditing Hanh's Shielded Zcash Ledger App
- Security Audit of zecwallet-lite-cli
- ZGo Security Audit Results
- Security Audit Process

**Research > Cryptography:**
- Making Zcash Light Wallets Faster and More Private (protocol design)
- Scalable Private Money Needs Scalable Anonymous Messaging
- Mitigating 51% Attack Risk on the Zcash Network

**Research > Other:**
- Risk Analysis of Intel's SGX and Other TEEs
- A Simple Threat Model for Zcash Shielded Hardware Wallets
- Security Engineering: Learning from Safety-Critical Disciplines (already in nav — place the zecsec version right next to it so the duplication is obvious for later cleanup)
- milk-sad (libbitcoin-explorer vulnerability)

**Not in nav** (project updates/transparency reports — accessible from the landing page but don't warrant a nav slot):
- Hello, World!
- October Update
- ZecSec Roadmap for 2023
- Q4 2022 Transparency Report
- Q1 2023 Transparency Report
- The Future of Zcash Ecosystem Security

The `zecsec.htm` landing page itself gets a nav link — location TBD (probably Research > Other or a top-level "ZecSec" entry).

### 10. KaTeX Considerations

The light wallets post has `katex: true` and uses `$...$` math notation. Since `$` is not a special character in CommonMark, comrak should pass it through as plain text. KaTeX auto-render then picks it up client-side. Potential issue: if markdown content between `$` delimiters contains `_` (subscripts) or `*` (multiplication), the markdown parser might interpret those as emphasis. Test this post specifically after rendering and fix any issues (e.g. by escaping `_` as `\_` in the pre-processed markdown if needed).
