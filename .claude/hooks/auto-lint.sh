#!/bin/bash
# Hook: PostToolUse (Edit|Write) — Auto-format & lint-fix the edited file only.
# Always exits 0 (non-blocking)

INPUT=$(cat)
PAYLOAD="$(dirname "$0")/lib/payload.cjs"

FILE_PATH=$(printf '%s' "$INPUT" | node "$PAYLOAD" file_path)

# Only lint TS/TSX/JS/JSX files
if [[ "$FILE_PATH" =~ \.(ts|tsx|js|jsx)$ ]]; then
  # Skip node_modules, dist, .venv
  if [[ "$FILE_PATH" == *"node_modules"* ]] || [[ "$FILE_PATH" == *"dist/"* ]] || [[ "$FILE_PATH" == *".venv"* ]]; then
    exit 0
  fi

  npx prettier --write "$FILE_PATH" 2>/dev/null
  npx eslint --fix "$FILE_PATH" 2>/dev/null
fi

exit 0
