#!/bin/bash
# Hook: PreToolUse (Edit|Write) — Blocks writing hardcoded secrets into source.
# Exit code 2 = block, 0 = allow.
#
# Barreira pontual no momento da escrita. Nao substitui gitleaks no pre-commit:
# este hook so ve o que o agente escreve.

INPUT=$(cat)
PAYLOAD="$(dirname "$0")/lib/payload.cjs"

FILE_PATH=$(printf '%s' "$INPUT" | node "$PAYLOAD" file_path)
CONTENT=$(printf '%s' "$INPUT" | node "$PAYLOAD" content)

block() { echo "BLOCKED: $1" >&2; exit 2; }

# .env.example carrega placeholders de proposito.
if [[ "$FILE_PATH" == *".env.example" ]]; then
  exit 0
fi

# Doc, regra e o proprio ferramental do agente CITAM o nome da chave o tempo
# todo (o guard precisa do padrao no fonte, o harness precisa dele no fixture).
# Nesses caminhos so o VALOR importa; a mencao do identificador e legitima.
IS_DOC=false
if [[ "$FILE_PATH" == *".md" ]] || [[ "$FILE_PATH" == *".claude/"* ]] || [[ "$FILE_PATH" == *".claude\\"* ]]; then
  IS_DOC=true
fi

# Credencial privilegiada referenciada em codigo de app
if [ "$IS_DOC" = false ] && echo "$CONTENT" | grep -qE 'SERVICE_ROLE_KEY|SERVICE_ACCOUNT_KEY|ADMIN_API_KEY'; then
  block "Conteudo referencia credencial privilegiada. O front usa apenas a chave publica/anon."
fi

# Atribuicao de chave hardcoded (JWT-like / sk- / chave longa) — vale em todo lugar
if echo "$CONTENT" | grep -qE '(apiKey|api_key|secret|token|password)\s*[:=]\s*["'\'']((eyJ[A-Za-z0-9_-]{20,})|(sk-[A-Za-z0-9]{20,})|([A-Za-z0-9_-]{40,}))["'\'']'; then
  block "Possivel chave/secret hardcoded. Use variavel de ambiente ou cofre da plataforma."
fi

# Chave privada colada em qualquer arquivo — vale em todo lugar
if echo "$CONTENT" | grep -qE 'BEGIN (RSA |EC |OPENSSH |PGP )?PRIVATE KEY'; then
  block "Chave privada detectada no conteudo. Nunca versionar chave privada."
fi

exit 0
