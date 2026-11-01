import { writeFileSync } from 'node:fs';
import { verifySourceSnapshot } from '../compat/voicegis/harness';
import { legacyRuntimeCases } from './legacy-runtime-cases';

if (!process.argv.includes('--record')) throw new Error('Pass --record to update expectations.');
verifySourceSnapshot();
writeFileSync('compat/voicegis/expected/runtime.json', JSON.stringify(await legacyRuntimeCases(), null, 2) + '\n');
