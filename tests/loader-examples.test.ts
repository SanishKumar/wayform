import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';
import { buildLoaderCases } from '../scripts/loader-cases';

test('native loading cases retain the draft validation boundary', () => {
  const recorded = JSON.parse(readFileSync(new URL('../packages/mobility-contract/examples/loading-v0.1.0-draft.1.json', import.meta.url), 'utf8'));
  assert.deepEqual(buildLoaderCases(), recorded);
});
