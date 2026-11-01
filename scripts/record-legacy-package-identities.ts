import { writeFileSync } from 'node:fs';
import { verifySourceSnapshot } from '../compat/voicegis/harness';
import { legacyPackageIdentityCases } from './legacy-package-identity-cases';

if (!process.argv.includes('--record')) throw new Error('Pass --record to update expectations.');
verifySourceSnapshot();
writeFileSync('compat/voicegis/expected/package-identities.json', JSON.stringify(legacyPackageIdentityCases(), null, 2) + '\n');
