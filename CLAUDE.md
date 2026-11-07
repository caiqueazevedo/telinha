# telinha — regras para agentes

Gravador de tela desktop para Windows: tela inteira, janela ou região, com microfone, áudio do
sistema e bolha de webcam sobreposta. Uso pessoal do Caique.

## Stack
- Tauri 2 (Rust 1.95, edition 2024) + React 19 + TypeScript 6 + Vite 8 (dev em `localhost:1420`)
- Testes: Vitest 5 + Testing Library (front), `cargo test` (back)
- Gerenciador: pnpm. Alvo: Windows apenas no MVP. Android foi adiado.

## Comandos
```bash
pnpm tauri dev                      # app em modo dev
pnpm typecheck                      # tsc --noEmit
pnpm test                           # vitest run
pnpm build                          # build do front (tsc + vite)
cd src-tauri && cargo clippy --all-targets -- -D warnings
cd src-tauri && cargo test
```

## Guardrails
`.claude/settings.json` liga hooks que bloqueiam de verdade. Os comandos reais
vivem em `.claude/quality.json`, não nesta prosa. Depois de mexer em hook:
`bash .claude/hooks/test-hooks.sh`

## Invariantes
_Vazio de propósito. Cada entrada nasce de um incidente: a regra, o motivo,
e o sintoma observável quando ela é violada._
