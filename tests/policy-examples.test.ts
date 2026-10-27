import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { test } from 'node:test';
import { buildPolicyCases } from '../scripts/policy-cases';

test('native policy fixtures retain TypeScript decisions and reason precedence', () => {
  const expected = readFileSync(new URL('../packages/mobility-contract/examples/policies-v0.1.0-draft.1.jsonl', import.meta.url), 'utf8').trim().split('\n').map(line => JSON.parse(line));
  assert.deepEqual(buildPolicyCases(), expected);
});
