# claude-idr

Generate Implementation Decision Records (IDR) from git diffs using Claude.

Automatically creates structured markdown documents that capture **what** changed, **why** it changed, and **what decisions** were made — at every commit.

## Features

- **Automatic IDR generation** at commit time via git pre-commit hook
- **Session-aware**: extracts context from Claude Code session logs
- **No jaq dependency**: uses built-in JSON parsing
- **Configurable**: language, model, output directory
- **Fail-open**: never blocks your commit

## Installation

### Homebrew (Recommended)

```bash
brew install thkt/tap/claude-idr
```

### From Release

```bash
# macOS (Apple Silicon)
curl -L https://github.com/thkt/claude-idr/releases/latest/download/claude-idr-aarch64-apple-darwin -o claude-idr
chmod +x claude-idr
mv claude-idr ~/.local/bin/
```

### From Source

```bash
git clone https://github.com/thkt/claude-idr.git
cd claude-idr
cargo build --release
cp target/release/claude-idr ~/.local/bin/
```

## Setup

Install as a git pre-commit hook in your project:

```bash
cd your-project
claude-idr-install  # or run: ./install.sh
```

Or add manually to `.git/hooks/pre-commit`:

```bash
claude-idr || true
```

## Usage

```bash
claude-idr [OPTIONS]

Options:
  --config <PATH>  Config file path
  --dry-run        Show prompt without calling claude
  --version        Show version
  --help           Show help
```

### How it works

1. Runs as a git pre-commit hook
2. Checks for recent Claude Code session with Write/Edit activity
3. Gets the staged diff (`git diff --cached`)
4. Extracts session context (changed files, user requests)
5. Calls Claude to generate an IDR with change summary and rationale
6. Writes `idr-NN.md` to the appropriate directory

### Output format

````markdown
# IDR: [purpose summary]

> 2026-02-07 17:30

## 変更概要
[One paragraph summary]

## 主要な変更
### [path/to/file](path/to/file)
#### L10-25: [change summary]
```diff
[diff hunk]
```

**理由**: [why this change was made]

## 設計判断

[Design decisions and rationale]

````

## Configuration

Create `~/.config/claude-idr/config.json`:

```json
{
  "enabled": true,
  "language": "ja",
  "model": "sonnet",
  "session_max_age_min": 30,
  "output_dir": null
}
```

| Option                | Default                 | Description                                               |
| --------------------- | ----------------------- | --------------------------------------------------------- |
| `enabled`             | `true`                  | Enable/disable IDR generation                             |
| `language`            | `"ja"`                  | Output language (`ja`, `en`)                              |
| `model`               | `"sonnet"`              | Claude model to use                                       |
| `session_max_age_min` | `30`                    | Max session age in minutes                                |
| `output_dir`          | `null`                  | Fixed IDR output directory (null = auto-resolve)          |
| `max_diff_lines`      | `500`                   | Max changed lines (additions+deletions); skip if exceeded |
| `workspace_dir`       | `"~/.claude/workspace"` | Workspace directory for SOW-aware resolution              |

### Output directory resolution

When `output_dir` is null (default), the output directory is resolved automatically:

1. Read `workspace_dir/.current-sow` for a SOW file path
2. If valid (exists, within workspace_dir), use the SOW file's parent directory
3. Otherwise, fall back to `workspace_dir/planning/YYYY-MM-DD/`

Config search order:

1. `--config` flag
2. `$XDG_CONFIG_HOME/claude-idr/config.json`
3. `~/.config/claude-idr/config.json`

## Requirements

- [Claude CLI](https://docs.anthropic.com/en/docs/claude-code) installed and authenticated
- git

## Exit Codes

| Code   | Meaning                            |
| ------ | ---------------------------------- |
| 0      | Success (IDR generated or skipped) |

The tool always exits 0 to never block commits (fail-open design).

## License

MIT
