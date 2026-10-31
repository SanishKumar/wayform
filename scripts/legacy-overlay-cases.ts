import { PINNED_COMMIT, loadVenue, readJson } from '../compat/voicegis/harness';
import type { CompiledBuildingPackage } from '../compat/voicegis/upstream/packages/map-compiler/src/compiler';
import { resolveOperationalOverlay, type OperationalOverlay } from '../compat/voicegis/upstream/src/engine/operationalOverlay';

function projectPackage(pkg: CompiledBuildingPackage) {
  return { buildingId: pkg.building.id, packageHash: pkg.manifest.contentHash, edges: pkg.routing.edges.map(({ id, sourceId }) => ({ id, sourceId })) };
}

export async function buildLegacyOverlayCases() {
  // The resolver reads only these package fields. This synthetic projection is
  // not passed off as a complete, verified venue package.
  const synthetic = {
    building: { id: 'fixture' }, manifest: { contentHash: 'a'.repeat(64) },
    routing: { edges: [
      { id: 'edge-\uE000', sourceId: 'lift' },
      { id: 'edge-\u{10000}', sourceId: 'lift' },
      { id: 'door-edge', sourceId: 'door' },
    ] },
  } as unknown as CompiledBuildingPackage;
  const base: OperationalOverlay = {
    schemaVersion: '0.1.0', id: 'ops', buildingId: 'fixture', packageHash: 'a'.repeat(64),
    validFrom: '2026-07-22T08:00:00.000Z', validUntil: '2026-07-23T00:00:00.000Z',
    closures: [
      { id: '\uE000', target: { type: 'source', id: 'lift' }, reason: 'Synthetic lift outage' },
      { id: '\u{10000}', target: { type: 'edge', id: 'door-edge' }, reason: 'Synthetic door closure' },
    ],
  };
  const now = '2026-07-22T12:00:00.000Z';
  type Input = { id: string; overlay: unknown; evaluatedAt: string };
  const cases: Input[] = [];
  const add = (id: string, overlay: unknown = structuredClone(base), evaluatedAt = now) => cases.push({ id, overlay, evaluatedAt });
  add('current-source-and-edge-utf16-order');
  add('inclusive-start', base, base.validFrom);
  add('before-start', base, '2026-07-22T07:59:59.999Z');
  add('before-expiry', base, '2026-07-22T23:59:59.999Z');
  add('exclusive-expiry', base, base.validUntil);
  add('after-expiry', base, '2026-07-23T00:00:00.001Z');
  add('missing-clock', base, '');
  add('invalid-clock', base, 'not-a-time');
  add('empty-closures', { ...base, closures: [] });
  add('source-only', { ...base, closures: [base.closures[0]] });
  add('edge-only', { ...base, closures: [base.closures[1]] });
  add('overlapping-targets', { ...base, closures: [...base.closures, { id: 'overlap', target: { type: 'edge', id: 'edge-\uE000' }, reason: 'Same closure' }] });
  for (const [id, fields] of [
    ['unsupported-version', { schemaVersion: 'future' }],
    ['building-mismatch', { buildingId: 'other' }],
    ['package-mismatch', { packageHash: 'b'.repeat(64) }],
    ['empty-overlay-id', { id: '' }],
    ['invalid-from', { validFrom: 'not-a-time' }],
    ['invalid-until', { validUntil: '2026-07-99T00:00:00.000Z' }],
    ['empty-time', { validUntil: '' }],
    ['empty-interval', { validUntil: base.validFrom }],
    ['reversed-interval', { validUntil: '2026-07-21T08:00:00.000Z' }],
  ] as const) add(id, { ...base, ...fields });
  for (const type of ['edge', 'source'] as const) {
    add(`unknown-${type}`, { ...base, closures: [...base.closures, { id: 'missing', target: { type, id: 'absent' }, reason: 'Unknown fixture target' }] });
  }
  add('duplicate-closure', { ...base, closures: [...base.closures, base.closures[0]] });
  add('duplicate-skips-invalid-target', { ...base, closures: [...base.closures, { ...base.closures[0], target: { type: 'edge', id: 'absent' } }] });
  add('ordered-multiple-issues', { ...base, schemaVersion: 'future', buildingId: 'other', packageHash: 'b'.repeat(64), closures: [
    base.closures[0], base.closures[0], { id: 'missing', target: { type: 'source', id: 'absent' }, reason: 'Fixture' },
  ] }, base.validUntil);
  add('extra-fields-retained-as-legacy-input', { ...base, extension: true, closures: base.closures.map(closure => ({ ...closure, extension: null, target: { ...closure.target, extension: 123 } })) });
  add('null-overlay', null);
  add('array-overlay', []);
  add('positional-overlay-array', [base.schemaVersion, base.id, base.buildingId, base.packageHash, base.validFrom, base.validUntil, base.closures]);
  add('malformed-with-id', { id: 'still-identifiable', closures: null });
  add('non-string-id', { ...base, id: 123 });
  add('non-array-closures', { ...base, closures: {} });
  add('null-closure', { ...base, closures: [null] });
  add('positional-closure-array', { ...base, closures: [['closure', { type: 'source', id: 'lift' }, 'Fixture']] });
  for (const key of ['schemaVersion', 'id', 'buildingId', 'packageHash', 'validFrom', 'validUntil', 'closures'] as const) {
    const missing = structuredClone(base) as Partial<OperationalOverlay>;
    delete missing[key];
    add(`missing-${key}`, missing);
  }
  for (const [id, closure] of [
    ['empty-closure-id', { ...base.closures[0], id: '' }],
    ['empty-reason', { ...base.closures[0], reason: '' }],
    ['non-string-reason', { ...base.closures[0], reason: 123 }],
    ['empty-target-id', { ...base.closures[0], target: { type: 'edge', id: '' } }],
    ['unknown-target-type', { ...base.closures[0], target: { type: 'building', id: 'fixture' } }],
    ['object-target-type', { ...base.closures[0], target: { type: { source: null }, id: 'lift' } }],
    ['numeric-target-type', { ...base.closures[0], target: { type: 1, id: 'lift' } }],
    ['positional-target-array', { ...base.closures[0], target: ['source', 'lift'] }],
    ['null-target', { ...base.closures[0], target: null }],
  ] as const) add(id, { ...base, closures: [closure] });
  const evaluated = (pkg: CompiledBuildingPackage, inputs: Input[]) => inputs.map(item => ({ ...item, expected: resolveOperationalOverlay(item.overlay, pkg, item.evaluatedAt) }));
  const pkg = await loadVenue('asterion-medical-center');
  const overlay = readJson<OperationalOverlay>('upstream/buildings/asterion-medical-center/operations/all-public-lifts-closed.overlay.json');
  const pinned: Input[] = [
    { id: 'current-lift-outage', overlay, evaluatedAt: now },
    { id: 'valid-from', overlay, evaluatedAt: overlay.validFrom },
    { id: 'expired', overlay, evaluatedAt: overlay.validUntil },
    { id: 'not-yet-active', overlay, evaluatedAt: '2026-07-22T07:59:59.999Z' },
    { id: 'missing-clock', overlay, evaluatedAt: '' },
    { id: 'wrong-package', overlay: { ...overlay, packageHash: '0'.repeat(64) }, evaluatedAt: now },
    { id: 'unknown-target', overlay: { ...overlay, closures: [{ id: 'unknown', target: { type: 'edge', id: 'missing' }, reason: 'fixture' }] }, evaluatedAt: now },
  ];
  return { formatVersion: 1, sourceCommit: PINNED_COMMIT, groups: [
    { id: 'synthetic', package: projectPackage(synthetic), cases: evaluated(synthetic, cases) },
    { id: 'asterion-medical-center', package: projectPackage(pkg), cases: evaluated(pkg, pinned) },
  ] };
}
