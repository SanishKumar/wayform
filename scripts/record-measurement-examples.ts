import { writeFileSync } from 'node:fs';
import { buildMeasurementCases } from './measurement-cases';

if (!process.argv.includes('--record')) throw new Error('Explicit --record required; expected decisions are review artifacts.');
const cases = buildMeasurementCases();
writeFileSync(new URL('../packages/mobility-contract/examples/measurements-v0.1.0-draft.1.jsonl', import.meta.url), cases.map(item => JSON.stringify(item)).join('\n') + '\n');
console.log(`Recorded ${cases.length} synthetic measurement cases from the TypeScript contract evaluator.`);
