#!/bin/bash
# Hook: PreToolUse (Bash) — Blocks destructive/unsafe shell commands.
# Exit code 2 = block, 0 = allow. Faz cumprir Rule 04 e Rule 13.

INPUT=$(cat)
PAYLOAD="$(dirname "$0")/lib/payload.cjs"

COMMAND=$(printf '%s' "$INPUT" | node "$PAYLOAD" command)

block() {
  echo "BLOCKED: $1" >&2
  exit 2
}

# rm -rf perigoso (raiz, home, ou recursivo amplo)
if echo "$COMMAND" | grep -qE 'rm\s+(-[a-zA-Z]*\s+)*-?[rRfF]+[a-zA-Z]*\s+(/|~|\$HOME|\.\s*$|\*\s*$)'; then
  block "rm -rf em caminho perigoso. Confirme manualmente."
fi

# git push --force em main/master
if echo "$COMMAND" | grep -qE 'git\s+push.*(--force|-f)\b' && echo "$COMMAND" | grep -qE '\b(main|master)\b'; then
  block "git push --force em main/master. Use --force-with-lease em branch, nunca force na main."
fi

# Contornar os hooks de qualidade (regra universal 7: investigar causa raiz)
if echo "$COMMAND" | grep -qE 'git\s+commit.*--no-verify'; then
  block "git commit --no-verify contorna os hooks de qualidade. Corrija a causa raiz."
fi

# DROP TABLE / DROP SCHEMA / DROP DATABASE (Rule 13: nunca dropar sem confirmacao)
if echo "$COMMAND" | grep -qiE 'drop\s+(table|schema|database)\b'; then
  block "DROP detectado. Rule 13: nunca dropar sem confirmacao explicita do usuario."
fi

# TRUNCATE em producao
if echo "$COMMAND" | grep -qiE 'truncate\s+table\b'; then
  block "TRUNCATE TABLE detectado. Confirme manualmente antes de apagar dados."
fi

# Vazamento de SERVICE_ROLE_KEY
if echo "$COMMAND" | grep -qE 'SERVICE_ROLE_KEY'; then
  block "Comando referencia SERVICE_ROLE_KEY. Rule 04: nunca expor a service role key."
fi

# Segredo como argumento vai para o historico do shell e para o log do provedor
if echo "$COMMAND" | grep -qE 'wrangler\s+secret\s+put\s+\S+\s+\S'; then
  block "Valor de segredo inline em 'wrangler secret put'. Passe por stdin — argumento vai para o historico do shell."
fi

# Codigo remoto executado sem revisao. Instalador de terceiro costuma reescrever
# a config do agente — justamente a superficie que estes guardrails protegem.
#
# O ancoramento em inicio de comando (^, ;, &&, ||, nova linha) e o que separa
# executar de CITAR: uma mensagem de commit que menciona 'curl | bash' entre
# aspas nao esta rodando nada. Sem a ancora este guard bloqueia a propria
# documentacao — aconteceu no commit que o introduziu.
if echo "$COMMAND" | grep -qE '(^|[;&|]|&&|\|\||[[:space:]]&&[[:space:]])[[:space:]]*(sudo[[:space:]]+)?(curl|wget|iwr|irm)[[:space:]][^|]*\|[[:space:]]*(sudo[[:space:]]+)?(bash|sh|zsh|iex|python[0-9.]*|node)\b'; then
  block "Execucao de codigo remoto sem revisao (curl|bash). Baixe, leia o script, rode em dois passos."
fi

exit 0
