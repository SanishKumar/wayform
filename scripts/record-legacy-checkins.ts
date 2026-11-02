import { writeFileSync } from 'node:fs';
import { verifySourceSnapshot } from '../compat/voicegis/harness';
import { legacyCheckinCases } from './legacy-checkin-cases';

if (!process.argv.includes('--record')) throw new Error('Pass --record to update expectations.');
verifySourceSnapshot();
writeFileSync('compat/voicegis/expected/checkins.json',JSON.stringify(await legacyCheckinCases(),null,2)+'\n');
