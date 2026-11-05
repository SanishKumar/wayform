import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { test } from 'node:test';
import { assessPath } from '../packages/mobility-contract/src/assess';
import { validateBundle } from '../packages/mobility-contract/src/validate';
import type { MobilityBundle } from '../packages/mobility-contract/src/types';
import { fixture } from './fixtures';

const bundle: MobilityBundle = JSON.parse(readFileSync('examples/survey/clinic-entrance.json', 'utf8'));

test('the clinic example distinguishes narrow, wide and unmeasured paths', () => {
  const { profile, context } = fixture();
  assert.deepEqual(validateBundle(bundle), []);
  profile.hard.minClearWidthMeters = 0.9;
  const assess = (edges: string[]) => assessPath(bundle, profile, context, 'Plaza entrance', 'Reception', edges);
  assert.equal(assess(['front-door']).status, 'unsuitable');
  const detour = ['side-door', 'lobby-corridor'];
  assert.equal(assess(detour).receipt.distanceMeters, 18);
  assert.equal(assess(detour).status, 'suitable');
  profile.hard.maxStepCount = 0;
  assert.equal(assess(detour).status, 'unknown');
  profile.hard.maxStepCount = null;
  context.evaluatedAt = '2026-11-01T00:00:00.000Z';
  assert.equal(assess(detour).status, 'unknown');
});
