import { writeFileSync } from 'node:fs';
import { buildIdentityCases } from './identity-cases';

if (!process.argv.includes('--record')) throw new Error('Explicit --record required; expected identities are review artifacts.');
const cases = buildIdentityCases();
writeFileSync(new URL('../packages/mobility-contract/examples/identities-v0.1.0-draft.1.jsonl', import.meta.url), cases.map(item => JSON.stringify(item)).join('\n') + '\n');
console.log(`Recorded ${cases.length} synthetic input-identity cases.`);
