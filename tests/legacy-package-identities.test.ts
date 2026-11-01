import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';
import { legacyPackageIdentityCases } from '../scripts/legacy-package-identity-cases';

test('legacy package identity retains the pinned pretty-print and key ordering', () => {
  assert.deepEqual(legacyPackageIdentityCases(), JSON.parse(readFileSync('compat/voicegis/expected/package-identities.json', 'utf8')));
});
