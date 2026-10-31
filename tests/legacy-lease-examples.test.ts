import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';
import { buildLegacyLeaseCases } from '../scripts/legacy-lease-cases';

test('legacy lease traces retain latched expiry, clock rules, activation and delay behavior', async () => {
  const recorded = JSON.parse(readFileSync(new URL('../compat/voicegis/expected/operational-leases.json', import.meta.url), 'utf8')) as Awaited<ReturnType<typeof buildLegacyLeaseCases>>;
  assert.deepEqual(await buildLegacyLeaseCases(), recorded);
  const cases = recorded.groups.flatMap(group => group.cases);
  const states = (id: string) => cases.find(item => item.id === id)!.expected.filter(value => typeof value !== 'number');
  assert.deepEqual(states('wall-expiry-stays-unavailable').map(value => value.status), ['current', 'unavailable', 'unavailable', 'unavailable']);
  assert.deepEqual(states('monotonic-expiry-with-frozen-wall').map(value => value.status), ['current', 'current', 'unavailable', 'unavailable']);
  assert.deepEqual(states('future-policy-activates-on-read').map(value => value.status), ['unavailable', 'unavailable', 'current', 'current', 'unavailable']);
  for (const item of cases) {
    let latched = false;
    for (const value of item.expected) {
      if (typeof value === 'number') {
        assert.ok(Number.isFinite(value) && value >= 1 && value <= 1000, item.id);
      } else {
        if (latched) assert.equal(value.status, 'unavailable', item.id);
        if (value.reason === 'clock-unreliable' || value.reason === 'overlay-expired') latched = true;
      }
    }
  }
});
