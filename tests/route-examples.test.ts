import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';
import { buildRouteCases } from '../scripts/route-cases';

test('native route fixtures retain contract receipts and reachability diagnostics', () => {
  const recorded = readFileSync(new URL('../packages/mobility-contract/examples/routes-v0.1.0-draft.1.jsonl', import.meta.url), 'utf8').trim().split('\n').map(line => JSON.parse(line));
  assert.deepEqual(buildRouteCases(), recorded);
});
