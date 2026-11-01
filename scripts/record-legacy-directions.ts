import { writeFileSync } from 'node:fs';
import { verifySourceSnapshot } from '../compat/voicegis/harness';
import { legacyDirectionCases } from './legacy-direction-cases';

if (!process.argv.includes('--record')) throw new Error('Pass --record to update expectations.');
verifySourceSnapshot();
writeFileSync('compat/voicegis/expected/base-directions.json', JSON.stringify(await legacyDirectionCases(), null, 2) + '\n');
writeFileSync('compat/voicegis/expected/landmark-directions.json', JSON.stringify(await legacyDirectionCases(true), null, 2) + '\n');
