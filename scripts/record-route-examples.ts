import { writeFileSync } from 'node:fs';
import { buildRouteCases } from './route-cases';
if (!process.argv.includes('--record')) throw new Error('Pass --record to replace route expectations.');
writeFileSync(new URL('../packages/mobility-contract/examples/routes-v0.1.0-draft.1.jsonl', import.meta.url), buildRouteCases().map(item => JSON.stringify(item)).join('\n') + '\n');
