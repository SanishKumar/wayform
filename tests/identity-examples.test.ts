import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { test } from 'node:test';
import { buildIdentityCases } from '../scripts/identity-cases';

test('native input identities retain ECMAScript serialization and collection normalization', () => {
  const expected = readFileSync(new URL('../packages/mobility-contract/examples/identities-v0.1.0-draft.1.jsonl', import.meta.url), 'utf8').trim().split('\n').map(line => JSON.parse(line));
  assert.deepEqual(JSON.parse(JSON.stringify(buildIdentityCases())), expected);
});
