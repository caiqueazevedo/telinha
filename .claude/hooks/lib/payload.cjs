// Extracts one field from a Claude Code hook payload (read from stdin).
//
// Usage: printf '%s' "$INPUT" | node .claude/hooks/lib/payload.js <field>
//   field: command | file_path | content | tool_name
//
// Reads fd 0 rather than '/dev/stdin': on Windows the latter resolves to
// <drive>:\dev\stdin and throws ENOENT, which silently emptied every hook
// that used it — they exited 0 and never blocked anything.
//
// The payload nests tool arguments under `tool_input`; older shapes put them
// at the root, so both are accepted.

const fs = require('fs');

let raw = '';
try {
    raw = fs.readFileSync(0, 'utf8');
} catch {
    process.exit(0);
}

let data;
try {
    data = JSON.parse(raw);
} catch {
    process.exit(0);
}

const input = data.tool_input ?? data;
const field = process.argv[2];

let value;
if (field === 'content') {
    // Write sends `content`, Edit sends `new_string`.
    value = input.content ?? input.new_string ?? '';
} else if (field === 'tool_name') {
    value = data.tool_name ?? '';
} else {
    value = input[field] ?? '';
}

process.stdout.write(String(value));
