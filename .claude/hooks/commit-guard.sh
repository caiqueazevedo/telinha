#!/bin/bash
# Hook: PreToolUse (Bash) — Portao de commit.
# 1. Valida o formato da mensagem contra .claude/quality.json (sempre bloqueia).
# 2. Roda os comandos listados em gate.commit.run (bloqueia se blocking=true).
#
# Exit 2 = bloqueia. Mensagem que nao da pra extrair com seguranca (heredoc,
# editor) passa: nao se bloqueia o que nao se conseguiu ler.

INPUT=$(cat)
DIR="$(dirname "$0")"
PAYLOAD="$DIR/lib/payload.cjs"
QUALITY="$DIR/lib/quality.cjs"

COMMAND=$(printf '%s' "$INPUT" | node "$PAYLOAD" command)

case "$COMMAND" in
  *"git commit"*) ;;
  *) exit 0 ;;
esac

block() { echo "BLOCKED: $1" >&2; exit 2; }

# --- 1. mensagem -------------------------------------------------------------
# Extrai o primeiro -m "..." ou -m '...'. Heredoc e $(...) nao sao parseaveis
# estaticamente e saem vazios de proposito.
MSG=$(printf '%s' "$COMMAND" | node -e "
  const c = require('fs').readFileSync(0,'utf8');
  const m = c.match(/-m\s+\"([^\"]*)\"/) || c.match(/-m\s+'([^']*)'/);
  process.stdout.write(m && !m[1].includes('\$(') ? m[1] : '');
")

if [ -n "$MSG" ]; then
  PROBLEM=$(printf '%s' "$MSG" | node "$QUALITY" check-commit 2>&1 >/dev/null)
  [ -n "$PROBLEM" ] && block "$PROBLEM"
fi

# --- 2. comandos configurados ------------------------------------------------
COMMANDS=$(node "$QUALITY" gate commit)
[ -z "$COMMANDS" ] && exit 0

BLOCKING=$(node "$QUALITY" blocking commit)
FAILED=""

while IFS= read -r cmd; do
  [ -z "$cmd" ] && continue
  if ! eval "$cmd" >/dev/null 2>&1; then
    FAILED="$FAILED\n  - $cmd"
  fi
done <<< "$COMMANDS"

if [ -n "$FAILED" ]; then
  if [ "$BLOCKING" = "true" ]; then
    block "Portao de qualidade falhou:$(printf '%b' "$FAILED")"
  fi
  node -e "
    const f = process.argv[1];
    console.log(JSON.stringify({hookSpecificOutput:{hookEventName:'PreToolUse',additionalContext:'[quality] Falhou antes do commit:'+f}}));
  " "$(printf '%b' "$FAILED")"
fi

exit 0
