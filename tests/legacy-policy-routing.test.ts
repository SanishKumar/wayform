import assert from 'node:assert/strict';
import test from 'node:test';
import { buildLegacyPolicyRouteCases } from '../scripts/legacy-policy-route-cases';

test('legacy policy routing comparisons block invalid overlays before endpoint handling', async () => {
  const batches = await buildLegacyPolicyRouteCases();
  for (const batch of batches) {
    assert.equal(batch.expected[0].outcome, 'routed');
    assert.equal(batch.expected.at(-1)!.outcome, 'routed');
    for (const result of batch.expected) {
      if (result.overlayResolution && !result.overlayResolution.valid) {
        assert.equal(result.outcome, 'rejected');
        assert.equal(result.error!.code, 'invalid-overlay');
        assert.equal(result.pathIds, undefined);
      }
    }
    for (const id of ['stationary-expired', 'unknown-endpoint-expired']) {
      assert.equal(batch.expected.find(result => result.queryId === id)!.outcome, 'rejected');
    }
  }
  const pinned = batches.find(batch => batch.request.requestId === 'policy/asterion-medical-center')!;
  assert.equal(pinned.expected.find(result => result.queryId === 'current-lift-outage/false')!.outcome, 'routed');
  assert.equal(pinned.expected.find(result => result.queryId === 'current-lift-outage/true')!.outcome, 'no-route');
});
