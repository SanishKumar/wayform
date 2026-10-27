import { writeFileSync } from 'node:fs';
import { buildPathCases } from './path-cases';
if (!process.argv.includes('--record')) throw new Error('Pass --record to replace path expectations.');
writeFileSync(new URL('../packages/mobility-contract/examples/paths-v0.1.0-draft.1.jsonl', import.meta.url), buildPathCases().map(value => JSON.stringify(value)).join('\n') + '\n');
