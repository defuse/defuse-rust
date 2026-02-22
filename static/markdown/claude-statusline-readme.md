![my statusline](https://raw.githubusercontent.com/defuse/claude-statusline/refs/heads/main/images/statusline-yellow.png)
![my statusline](https://raw.githubusercontent.com/defuse/claude-statusline/refs/heads/main/images/statusline.png)

### Installation

Add these scripts to `~/.claude` and add the `statusline.sh` script to your `~/.claude/settings.json`, e.g:

```json
{
  "model": "opus",
  "statusLine": {
    "type": "command",
    "command": "~/.claude/statusline.sh"
  }
}
```

To add the scripts to `~/.claude` (assuming you don't have a git repo there already):

```
cd ~/.claude
git init
git remote add origin git@github.com:defuse/claude-statusline.git
git fetch
git checkout -t origin/main
```

### Dependencies

- `jq` - JSON parsing
- `python3` with `pty` module (stdlib) - used by `autocompact-buffer.sh` to probe Claude Code's `/context` command in a fake TTY
- `/tmp` must be a trusted workspace in Claude Code (run `claude` once in `/tmp` and accept the trust prompt). The probe runs there to avoid polluting session history in your actual projects.
- `git` - branch display (optional, degrades gracefully)
- `~/.claude/.credentials.json` must contain your API credentials (it should by default)
