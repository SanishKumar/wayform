import { writeFileSync } from 'node:fs';
import { buildLoaderCases } from './loader-cases';
if (!process.argv.includes('--record')) throw new Error('Pass --record to replace loader expectations.');
writeFileSync(new URL('../packages/mobility-contract/examples/loading-v0.1.0-draft.1.json', import.meta.url), JSON.stringify(buildLoaderCases(), null, 2) + '\n');
