import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import path from 'node:path';
import { test } from 'node:test';
import { buildLegacyBaseline, root, verifySourceSnapshot } from '../compat/voicegis/harness';

test('pinned source has the exact upstream Git blob bytes and inventory', () => {
  assert.equal(verifySourceSnapshot(), 35);
});
test('legacy journeys, compiler artifacts, receipts and negative outcomes match the recorded baseline', async () => {
  const actual = await buildLegacyBaseline();
  for (const [name, contents] of Object.entries(actual)) {
    const expected = readFileSync(path.join(root, 'expected', name), 'utf8');
    if (name.endsWith('.jsonl')) {
      const rows = contents.trim().split('\n');
      const oldRows = expected.trim().split('\n');
      assert.equal(rows.length, oldRows.length);
      rows.forEach((row, index) => assert.equal(row, oldRows[index], `Legacy route changed at row ${index + 1}: ${JSON.parse(row).id}`));
    } else assert.equal(contents, expected, name);
  }
});
