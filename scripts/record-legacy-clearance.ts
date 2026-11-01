import { writeFileSync } from 'node:fs';
import { verifySourceSnapshot } from '../compat/voicegis/harness';
import { legacyClearanceCorpus, legacyClearanceNegativeCases } from './legacy-clearance-cases';

if (!process.argv.includes('--record')) throw new Error('Pass --record to update expectations.');
verifySourceSnapshot();
writeFileSync('compat/voicegis/expected/clearance.json', JSON.stringify({ corpus: await legacyClearanceCorpus(), negative: await legacyClearanceNegativeCases() }, null, 2) + '\n');
