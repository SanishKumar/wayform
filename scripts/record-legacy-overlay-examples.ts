import { writeFileSync } from 'node:fs';
import { verifySourceSnapshot } from '../compat/voicegis/harness';
import { buildLegacyOverlayCases } from './legacy-overlay-cases';
if (!process.argv.includes('--record')) throw new Error('Pass --record to replace legacy overlay expectations.');
verifySourceSnapshot();
writeFileSync(new URL('../compat/voicegis/expected/operational-overlays.json', import.meta.url), JSON.stringify(await buildLegacyOverlayCases(), null, 2) + '\n');
