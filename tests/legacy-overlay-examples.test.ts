import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';
import { buildLegacyOverlayCases } from '../scripts/legacy-overlay-cases';

test('legacy overlay fixtures retain closure expansion, issue ordering and expiry boundaries', async () => {
  const recorded = JSON.parse(readFileSync(new URL('../compat/voicegis/expected/operational-overlays.json', import.meta.url), 'utf8')) as Awaited<ReturnType<typeof buildLegacyOverlayCases>>;
  assert.deepEqual(await buildLegacyOverlayCases(), recorded);
  const synthetic = recorded.groups[0].cases;
  const current = synthetic.find(item => item.id === 'current-source-and-edge-utf16-order')!.expected;
  assert.equal(current.valid, true);
  assert.deepEqual(current.activeClosureIds, ['\u{10000}', '\uE000']);
  assert.deepEqual(current.closedEdgeIds, ['door-edge', 'edge-\u{10000}', 'edge-\uE000']);
  const multiple = synthetic.find(item => item.id === 'ordered-multiple-issues')!.expected;
  assert.deepEqual(multiple.issues.map(issue => issue.code), ['invalid-schema-version', 'building-mismatch', 'package-mismatch', 'overlay-expired', 'duplicate-closure', 'unknown-target']);
  assert.ok(synthetic.some(item => item.id === 'exclusive-expiry' && !item.expected.valid));
  for (const group of recorded.groups) for (const item of group.cases) {
    if (!item.expected.valid) {
      assert.ok(item.expected.issues.length > 0, item.id);
      assert.deepEqual(item.expected.activeClosureIds, [], item.id);
      assert.deepEqual(item.expected.closedEdgeIds, [], item.id);
    }
  }
});
