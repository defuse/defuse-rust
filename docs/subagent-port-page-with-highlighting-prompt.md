# Subagent Prompt for Porting Pages with Vim Highlighting

Use this prompt for pages that contain `printSourceFile` and/or `printHlString` PHP calls.

Replace `[SLUG]` with the actual page slug.

---

You are porting a page from PHP to Rust that contains vim syntax highlighting calls. Work on the page with slug "[SLUG]".

## Reference Implementation

First, read `/home/taylor/defuse-rewrite/defuse-rust/templates/pages/research/blind_birthday_attack.html` to see the pattern for `ctx.hl_file` and `ctx.hl_string` usage.

## Step 1: Get metadata from URLParse.php

Read `/home/taylor/defuse-rewrite/defuse.ca/src/libs/URLParse.php` and find the entry for "[SLUG]" in the `$PAGE_INFO` array. Extract:
- P_FILE (index 0) - the file path relative to pages/
- P_TITL (index 1) - title
- P_METD (index 2) - meta description
- P_METK (index 3) - meta keywords

Parse P_FILE to extract SUBDIR and FILENAME as in the standard prompt.

## Step 2: Copy and wrap the template

Run these EXACT commands (do NOT skip this step):
```bash
cp "/home/taylor/defuse-rewrite/defuse.ca/src/pages/[P_FILE]" /tmp/[HANDLER_NAME]_raw.html
/home/taylor/defuse-rewrite/defuse-rust/scripts/wrap_template.sh /tmp/[HANDLER_NAME]_raw.html /home/taylor/defuse-rewrite/defuse-rust/templates/pages/[SUBDIR]/[HANDLER_NAME].html
```

## Step 3: Edit the template with SURGICAL replacements

Use the Edit tool to make targeted replacements in the copied file. Do NOT rewrite the file from scratch.

### 3a. Remove Upvote::render_arrows block

Find and delete the Upvote block. The replacement must join `{% block content %}` directly to the first HTML element (could be `<div class="pagedate">`, `<h1>`, or any other tag):

```
OLD: {% block content %}<?php
    Upvote::render_arrows(...);
?>
<h1>Page Title</h1>

NEW: {% block content %}<h1>Page Title</h1>
```

Note: Delete from `<?php` through `?>` AND the newline after `?>`.

### 3b. Replace printSourceFile calls

Each `printSourceFile` call spans 3 lines. Replace the ENTIRE block:

```
OLD: <?php
    printSourceFile("source/Example.cpp", true);
?>

NEW: {{ ctx.hl_file("static/source/Example.cpp", true)|safe }}
```

**CRITICAL**: PHP's `?>` consumes the newline that follows it. This means:
- The `ctx.hl_*` call should NEVER have a blank line after it
- The next element (`<h2>`, `<p>`, `{% endblock %}`, etc.) goes on the very next line
- If the PHP had a blank line BEFORE the `<?php`, keep that blank line before the `ctx.hl_*` call

Example showing correct output:
```
{{ ctx.hl_file("static/source/Example.cpp", true)|safe }}
<h2>Next Section</h2>
```
NOT:
```
{{ ctx.hl_file("static/source/Example.cpp", true)|safe }}

<h2>Next Section</h2>
```

### 3c. Replace printHlString calls

`printHlString` can use either heredoc or inline string syntax.

**Heredoc syntax** (spans multiple lines with `<<<EOT`):

```
OLD: <?php
    $str = <<<EOT
content line 1
content line 2
EOT;
    printHlString($str, "text", false);
?>

NEW: {{ ctx.hl_string("content line 1
content line 2", "text", false)|safe }}
```

**Inline string syntax** (single-quoted multi-line string):

```
OLD: <?php
$source = '<?php
require_once(\'PasswordGenerator.php\');
echo "hello";
?>';
printHlString($source, "php", true);
?>

NEW: {{ ctx.hl_string("<?php
require_once('PasswordGenerator.php');
echo \"hello\";
?>", "php", true)|safe }}
```

Note for inline strings:
- Convert `\'` (escaped single quotes) to plain `'`
- Convert `"` to `\"` (escape double quotes for the Jinja string)
- Preserve all other content exactly

**CRITICAL whitespace rules**:
1. Copy string content EXACTLY - preserve all tabs, spaces, internal newlines
2. If the string ends with a blank line, include that trailing newline
3. The `ctx.hl_string` call should NEVER have a blank line after it (PHP's `?>` consumes newlines)
4. The next element goes on the very next line after the `|safe }}` closing

### 3d. Handle consecutive PHP blocks

When two PHP blocks are adjacent (no blank line between `?>` and next `<?php`), the replacements should also be adjacent:

```
OLD: <?php
    printSourceFile("source/A.cpp", true);
?>
<?php
    $str = <<<EOT
output
EOT;
    printHlString($str, "text", false);
?>

NEW: {{ ctx.hl_file("static/source/A.cpp", true)|safe }}
{{ ctx.hl_string("output", "text", false)|safe }}
```

When there IS a blank line between PHP blocks, keep one blank line between replacements.

### 3e. Delete standalone PHP comments

Delete any PHP blocks that are just comments:
```php
<?php
// comment here
?>
```

### 3f. Verify whitespace structure

After all edits, the template should have:
- `{% block content %}` on same line as first HTML tag
- **NO blank lines after `ctx.hl_file` or `ctx.hl_string` calls** - next element on immediate next line
- If the last thing before `{% endblock %}` is a `ctx.hl_*` call, put `{% endblock %}` on the SAME line: `...|safe }}{% endblock %}`
- No extra blank lines introduced elsewhere
- Blank lines BEFORE `ctx.hl_*` calls are OK if the original had them

## Step 4: Create the handler

Create `/home/taylor/defuse-rewrite/defuse-rust/src/pages/[SUBDIR]/[HANDLER_NAME].rs`:
```rust
crate::simple_page!([PageName]Page, "pages/[SUBDIR]/[HANDLER_NAME].html");
```

## Step 5: Update mod.rs

Add `pub mod [HANDLER_NAME];` to the appropriate mod.rs in alphabetical order.

## Step 6: Add to registry

Add a page! entry to `/home/taylor/defuse-rewrite/defuse-rust/src/registry/pages.rs` with:
- Metadata from URLParse.php (title, description, keywords, legacy_hit_count_id)
- Upvote config extracted from the Upvote::render_arrows call (if present)

## Step 7: Verify

1. Run `cargo check` in `/home/taylor/defuse-rewrite/defuse-rust`
2. Check the box in `/home/taylor/defuse-rewrite/defuse-tester/docs/pages-checklist.md`

Report exactly what edits you made and verify the whitespace structure matches the original.
