import { writeFileSync } from 'node:fs';
import { verifySourceSnapshot } from '../compat/voicegis/harness';
import { buildLegacySearchCases } from './legacy-search-cases';
if (!process.argv.includes('--record')) throw new Error('Pass --record to replace legacy search expectations.');
verifySourceSnapshot();
writeFileSync(new URL('../compat/voicegis/expected/search-kernel.jsonl', import.meta.url), buildLegacySearchCases().map(item => JSON.stringify(item)).join('\n') + '\n');
