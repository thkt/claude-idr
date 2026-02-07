#!/bin/bash
# Install claude-idr as git pre-commit hook
set -euo pipefail

command -v claude-idr &>/dev/null || {
  echo "claude-idr not found. Install: brew install thkt/tap/claude-idr"
  exit 1
}

HOOK_DIR="$(git rev-parse --git-dir 2>/dev/null)/hooks" || {
  echo "Not in a git repository"
  exit 1
}

HOOK_FILE="$HOOK_DIR/pre-commit"

if [ -f "$HOOK_FILE" ]; then
  if grep -q "claude-idr" "$HOOK_FILE"; then
    echo "claude-idr already installed in pre-commit hook"
    exit 0
  fi
  echo "" >> "$HOOK_FILE"
  echo "# claude-idr: Implementation Decision Record generator" >> "$HOOK_FILE"
  echo "claude-idr || true" >> "$HOOK_FILE"
else
  cat > "$HOOK_FILE" << 'EOF'
#!/bin/bash
# claude-idr: Implementation Decision Record generator
claude-idr || true
EOF
  chmod +x "$HOOK_FILE"
fi

echo "claude-idr installed in $(realpath "$HOOK_FILE")"
