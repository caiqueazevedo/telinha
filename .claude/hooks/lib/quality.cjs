// Lê .claude/quality.json — o manifesto de qualidade do repo.
//
// Existe porque os comandos reais divergem por projeto (`npx tsc -b` onde há
// project references quebra com `--noEmit`; `corepack pnpm` onde o
// packageManager está fixado). Prosa não serve: o hook precisa executar sem
// interpretar. O manifesto é a fonte única lida pelo /verify e pelos hooks.
//
// Uso:
//   node quality.cjs gate <stop|commit>   -> um comando por linha (só os definidos)
//   node quality.cjs blocking <stop|commit> -> "true" | "false"
//   node quality.cjs check-commit          -> mensagem em stdin; exit 1 + motivo em stderr
//
// Sem manifesto, todo comando sai vazio e todo check passa: repo não
// configurado não é repo bloqueado.

const fs = require('fs');
const path = require('path');

const MANIFEST = path.join(process.cwd(), '.claude', 'quality.json');

function load() {
    try {
        return JSON.parse(fs.readFileSync(MANIFEST, 'utf8'));
    } catch {
        return null;
    }
}

const DEFAULT_TYPES = ['feat', 'fix', 'docs', 'style', 'refactor', 'test', 'chore', 'perf', 'ci'];

function gateCommands(cfg, gate) {
    const names = cfg?.gate?.[gate]?.run ?? [];
    return names.map((n) => cfg?.commands?.[n]).filter((c) => typeof c === 'string' && c.length > 0);
}

function checkCommit(cfg, message) {
    const subject = message.split('\n')[0].trim();
    if (!subject) return null; // nada a validar

    if (/Co-Authored-By:/i.test(message)) {
        return 'A mensagem contem Co-Authored-By. Regra universal: sem coautoria.';
    }

    const rules = cfg?.commit ?? {};
    const types = rules.types ?? DEFAULT_TYPES;
    const max = rules.subject_max ?? 72;

    // <tipo>(<escopo opcional>)<! opcional>: <descricao>
    const m = subject.match(/^([a-z]+)(\(([^)]+)\))?(!)?: (.+)$/);
    if (!m) {
        return `Formato invalido: "${subject}". Esperado "<tipo>(<escopo>): <descricao>" com tipo em ${types.join(', ')}.`;
    }

    const [, type, , scope] = m;
    // Tipo aposentado em favor de outro — mesma logica dos escopos, um nivel
    // acima. A mensagem aponta o canonico em vez de so recusar, porque tipo
    // errado quase sempre e habito antigo, nao desconhecimento da regra.
    if (rules.type_aliases && rules.type_aliases[type]) {
        return `Tipo "${type}" foi aposentado neste repo; use "${rules.type_aliases[type]}".`;
    }
    if (!types.includes(type)) {
        return `Tipo "${type}" nao permitido neste repo. Tipos: ${types.join(', ')}.`;
    }
    if (rules.scope_required && !scope) {
        return `Escopo obrigatorio neste repo: "<tipo>(<escopo>): <descricao>".`;
    }
    // Variante deprecada de um escopo que ja existe com outro nome. Existe
    // porque o mesmo conceito forkou entre PT e EN (perfil/profile,
    // media/midia) e partiu o historico em dois — pior que qualquer das duas
    // escolhas isoladamente. Bloqueia a variante e aponta a canonica.
    if (rules.scope_aliases && scope && rules.scope_aliases[scope]) {
        return `Escopo "${scope}" foi aposentado neste repo; use "${rules.scope_aliases[scope]}". Duas grafias para o mesmo conceito partem o historico.`;
    }
    if (rules.scopes && scope && !rules.scopes.includes(scope)) {
        return `Escopo "${scope}" nao esta na lista do repo: ${rules.scopes.join(', ')}.`;
    }
    if (subject.length > max) {
        return `Assunto com ${subject.length} chars; maximo ${max}.`;
    }
    return null;
}

const [, , cmd, arg] = process.argv;
const cfg = load();

if (cmd === 'gate') {
    if (cfg) process.stdout.write(gateCommands(cfg, arg).join('\n'));
    process.exit(0);
}

if (cmd === 'blocking') {
    process.stdout.write(cfg?.gate?.[arg]?.blocking === true ? 'true' : 'false');
    process.exit(0);
}

if (cmd === 'check-commit') {
    let message = '';
    try {
        message = fs.readFileSync(0, 'utf8');
    } catch {
        process.exit(0);
    }
    const problem = checkCommit(cfg, message);
    if (problem) {
        process.stderr.write(problem);
        process.exit(1);
    }
    process.exit(0);
}

process.stderr.write(`uso: quality.cjs gate|blocking|check-commit`);
process.exit(2);
