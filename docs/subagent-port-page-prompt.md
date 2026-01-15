# Subagent Prompt for Porting Static Pages

Replace `[SLUG]` with the actual page slug.

---

You are porting a single page from a PHP codebase to a Rust rewrite. Work on the page with slug "[SLUG]".

**Step 1: Get metadata from URLParse.php**
Read `/home/taylor/defuse-rewrite/defuse.ca/src/libs/URLParse.php` and find the entry for "[SLUG]" in the `$PAGE_INFO` array. Extract:
- P_FILE (index 0) - the file path relative to pages/
- P_TITL (index 1) - title, use "" if missing
- P_METD (index 2) - meta description, use "" if missing
- P_METK (index 3) - meta keywords, use "" if missing

**Step 1b: Parse directory structure from P_FILE**
Parse P_FILE to extract the subdirectory and filename:
- If P_FILE contains a `/` (e.g., `software/sockstress.php`):
  - SUBDIR = the directory part (e.g., `software`)
  - FILENAME = the file part (e.g., `sockstress.php`)
- If P_FILE has no `/` (e.g., `home.html`):
  - SUBDIR = "" (empty, no subdirectory)
  - FILENAME = the entire P_FILE

**Step 2: Read the source file**
Read the file at `/home/taylor/defuse-rewrite/defuse.ca/src/pages/` + P_FILE

**Step 3: Check for executable PHP**
Look for `<?` in the file.

- If the file contains NO `<?` tags: proceed to Step 4.

- If the file contains `<?` with ONLY `Upvote::render_arrows(...)` and no other PHP code:
  - Extract the upvote metadata from the call. The format is:
    ```
    Upvote::render_arrows(
        "id",
        "category",
        "title",
        "description",
        "url"
    );
    ```
  - Note these values: UPVOTE_ID, UPVOTE_CATEGORY, UPVOTE_TITLE, UPVOTE_DESCRIPTION
  - Set HAS_UPVOTE = true
  - Proceed to Step 4.

- If the file contains `<?` with ANY OTHER PHP code (not just upvote arrows):
  - Edit `/home/taylor/defuse-rewrite/defuse-tester/docs/pages-checklist.md` to check the box for "[SLUG]"
  - STOP and report: "Skipped: contains executable PHP (not just upvote arrows)"

**Step 4: Check if already in Rust**
Read `/home/taylor/defuse-rewrite/defuse-rust/src/registry/pages.rs` and check if "[SLUG]" already exists as a page slug.
If it exists:
- Edit `/home/taylor/defuse-rewrite/defuse-tester/docs/pages-checklist.md` to check the box for "[SLUG]"
- STOP and report: "Skipped: already exists in Rust"

**Step 5: Port the page**
If you reach this step:

First, derive HANDLER_NAME from the slug by replacing all hyphens with underscores.
Example: "security-contact-vulnerability-disclosure" → "security_contact_vulnerability_disclosure"

Determine the full paths based on SUBDIR:
- If SUBDIR is empty:
  - TEMPLATE_PATH = `templates/pages/[HANDLER_NAME].html`
  - HANDLER_PATH = `src/pages/[HANDLER_NAME].rs`
  - TEMPLATE_REF = `pages/[HANDLER_NAME].html`
  - MOD_PATH = `src/pages/mod.rs`
  - HANDLER_REF = `[HANDLER_NAME]` (for registry)
- If SUBDIR is not empty:
  - TEMPLATE_PATH = `templates/pages/[SUBDIR]/[HANDLER_NAME].html`
  - HANDLER_PATH = `src/pages/[SUBDIR]/[HANDLER_NAME].rs`
  - TEMPLATE_REF = `pages/[SUBDIR]/[HANDLER_NAME].html`
  - MOD_PATH = `src/pages/[SUBDIR]/mod.rs`
  - HANDLER_REF = `[SUBDIR]::[HANDLER_NAME]` (for registry, e.g., `software::sockstress`)

1. **Create subdirectories if needed** (only if SUBDIR is not empty):
   ```
   mkdir -p /home/taylor/defuse-rewrite/defuse-rust/templates/pages/[SUBDIR]
   mkdir -p /home/taylor/defuse-rewrite/defuse-rust/src/pages/[SUBDIR]
   ```

2. **Copy and wrap the template** using these EXACT commands:
   ```
   cp "/home/taylor/defuse-rewrite/defuse.ca/src/pages/[P_FILE]" /tmp/[HANDLER_NAME]_raw.html
   /home/taylor/defuse-rewrite/defuse-rust/scripts/wrap_template.sh /tmp/[HANDLER_NAME]_raw.html /home/taylor/defuse-rewrite/defuse-rust/[TEMPLATE_PATH]
   ```

3. **If HAS_UPVOTE is true**: Edit the template file to delete the `<?php ... ?>` block containing the Upvote::render_arrows call AND the newline that follows `?>`. The goal is that `{% block content %}` ends up on the SAME LINE as the first real HTML content (e.g., `{% block content %}<h1>...`). Do NOT leave a blank line between `{% block content %}` and the first HTML tag.

4. **Create the handler file** at `/home/taylor/defuse-rewrite/defuse-rust/[HANDLER_PATH]` with ONLY this content:
   ```
   crate::simple_page!([PageName]Page, "[TEMPLATE_REF]");
   ```
   Where [PageName] is the HANDLER_NAME in PascalCase (e.g., "services" → "Services", "backup_verify_script" → "BackupVerifyScript").

5. **Add to mod.rs**:
   - If SUBDIR is empty:
     - Add `pub mod [HANDLER_NAME];` to `/home/taylor/defuse-rewrite/defuse-rust/src/pages/mod.rs` in alphabetical order.
   - If SUBDIR is not empty:
     - First, check if `/home/taylor/defuse-rewrite/defuse-rust/src/pages/[SUBDIR]/mod.rs` exists:
       - If it does NOT exist, create it with content: `pub mod [HANDLER_NAME];`
       - If it DOES exist, add `pub mod [HANDLER_NAME];` to it in alphabetical order.
     - Then, check if `pub mod [SUBDIR];` exists in `/home/taylor/defuse-rewrite/defuse-rust/src/pages/mod.rs`:
       - If not, add `pub mod [SUBDIR];` in alphabetical order.

6. **Add to registry**: Add a page! entry to `/home/taylor/defuse-rewrite/defuse-rust/src/registry/pages.rs`.

   If HAS_UPVOTE is false:
   ```
   page! {
       handler: [HANDLER_REF],
       slug: "[SLUG]",
       title: "[P_TITL from Step 1]",
       description: "[P_METD from Step 1]",
       keywords: "[P_METK from Step 1]",
       legacy_hit_count_id: "pages/[P_FILE from Step 1]",
       upvote: None,
   },
   ```

   If HAS_UPVOTE is true:
   ```
   page! {
       handler: [HANDLER_REF],
       slug: "[SLUG]",
       title: "[P_TITL from Step 1]",
       description: "[P_METD from Step 1]",
       keywords: "[P_METK from Step 1]",
       legacy_hit_count_id: "pages/[P_FILE from Step 1]",
       upvote: Some(UpvoteConfig {
           id: "[UPVOTE_ID]",
           category: "[UPVOTE_CATEGORY]",
           title: Some("[UPVOTE_TITLE]"),
           description: Some("[UPVOTE_DESCRIPTION]"),
       }),
   },
   ```

7. **Check the box** in `/home/taylor/defuse-rewrite/defuse-tester/docs/pages-checklist.md`

8. **Verify the build**: Run `cargo check` in `/home/taylor/defuse-rewrite/defuse-rust`

Report back exactly what you did and why.
