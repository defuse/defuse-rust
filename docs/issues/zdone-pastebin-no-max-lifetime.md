# Pastebin Missing Maximum Lifetime Validation

## Bug

The pastebin accepts arbitrary lifetime values with no server-side validation. The HTML form limits options to 6 months maximum, but this is trivially bypassed.

## Current Behavior

```php
// add.php line 25
(isset($_POST['lifetime']) ? (int)$_POST['lifetime'] : 3600*24*10)
```

Someone can POST `lifetime=315360000` (10 years) or any large value and it will be accepted.

## Expected Behavior

Server should validate that lifetime is one of the allowed values:
- 600 (10 minutes)
- 3600 (60 minutes)
- 86400 (1 day)
- 864000 (10 days)
- 2592000 (30 days)
- 15552000 (6 months)

Or at minimum, clamp to max 6 months (15552000 seconds).

## Fix for Rust Port

Validate lifetime server-side. Reject or clamp values outside allowed range.

```rust
const MAX_LIFETIME: i64 = 15552000; // 6 months
let lifetime = lifetime.min(MAX_LIFETIME).max(60); // At least 1 minute, at most 6 months
```

## Impact

Low - pastes just last longer than intended. No security impact, just resource usage.
