import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { readFileSync, readdirSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import path from 'node:path';
import { compileBuilding, stableJson, type CompiledBuildingPackage } from './upstream/packages/map-compiler/src/compiler';
import { verifyVenuePackage } from './upstream/src/data/venuePackageContract';
import { createCompiledBuildingRuntime } from './upstream/src/data/compiledBuilding';
import { calculateCompiledRoute, type CompiledRouteOptions } from './upstream/src/engine/compiledRoutePolicy';
import { checkInFromScan, scannableAnchors } from './upstream/src/capture/anchorCheckIn';
import { createOperationalLease } from './upstream/src/navigation/operationalLease';
import type { OperationalOverlay } from './upstream/src/engine/operationalOverlay';

export const PINNED_COMMIT = '305c70bd418d7fd2b6f0ebf7a2ecd0ddcb9b9b7f';
export const root = fileURLToPath(new URL('./', import.meta.url));
export const venues = ['reference-medical-centre', 'asterion-medical-center', 'harbor-exchange', 'meridian-park-campus'] as const;
const expectedCounts = [242, 2496, 200, 4300];
export const sha256 = (bytes: string | Buffer) => createHash('sha256').update(bytes).digest('hex');
export const readJson = <T = unknown>(relative: string): T => JSON.parse(readFileSync(path.join(root, relative), 'utf8'));
// Independent of object insertion order; array order and exact numeric behavior are preserved.
export function canonical(value: unknown): string {
  const order = (input: unknown): unknown => Array.isArray(input) ? input.map(order)
    : input !== null && typeof input === 'object'
      ? Object.fromEntries(Object.entries(input).sort(([a], [b]) => a < b ? -1 : a > b ? 1 : 0).map(([key, child]) => [key, order(child)]))
      : input;
  return JSON.stringify(order(value));
}
export function verifySourceSnapshot() {
  const lock = readJson<{ commit: string; files: { path: string; bytes: number; sha256: string; gitBlob: string }[] }>('source-lock.json');
  assert.equal(lock.commit, PINNED_COMMIT);
  const inventory = (directory: string): string[] => readdirSync(directory, { withFileTypes: true }).flatMap(entry => {
    const full = path.join(directory, entry.name);
    return entry.isDirectory() ? inventory(full) : [path.relative(path.join(root, 'upstream'), full).replaceAll('\\', '/')];
  });
  assert.deepEqual(inventory(path.join(root, 'upstream')).sort(), lock.files.map(file => file.path).sort(), 'Pinned file inventory changed');
  for (const file of lock.files) {
    const bytes = readFileSync(path.join(root, 'upstream', file.path));
    assert.equal(bytes.length, file.bytes, file.path);
    assert.equal(sha256(bytes), file.sha256, `Source digest changed: ${file.path}`);
    const blob = createHash('sha1').update(`blob ${bytes.length}\0`).update(bytes).digest('hex');
    assert.equal(blob, file.gitBlob, `Git blob changed: ${file.path}`);
  }
  return lock.files.length;
}
export async function loadVenue(venue: string) {
  return verifyVenuePackage(readJson(`upstream/buildings/${venue}/compiled/building.package.json`));
}

export async function buildLegacyBaseline() {
  verifySourceSnapshot();
  const routeDigests: string[] = [];
  const summaries = [];
  const examples: Record<string, unknown> = {};
  for (const [venueIndex, venue] of venues.entries()) {
    const pkg = await loadVenue(venue);
    const source = readJson(`upstream/buildings/${venue}/source/building.json`);
    const first = compileBuilding(source);
    const second = compileBuilding(structuredClone(source));
    assert.equal(first.report.valid, true);
    assert.equal(stableJson(first), stableJson(second), `${venue}: nondeterministic compilation`);
    assert.equal(stableJson(first.package), readFileSync(path.join(root, `upstream/buildings/${venue}/compiled/building.package.json`), 'utf8'), `${venue}: package bytes changed`);
    assert.equal(stableJson(first.report), readFileSync(path.join(root, `upstream/buildings/${venue}/compiled/validation-report.json`), 'utf8'), `${venue}: report bytes changed`);
    const runtime = createCompiledBuildingRuntime(pkg);
    const places = runtime.getPOIs().map(poi => poi.id);
    const starts = places.map(id => ({ label: id, id }));
    for (const anchor of scannableAnchors(pkg.localizationAnchors)) {
      const scan = checkInFromScan(anchor.payload, pkg.localizationAnchors, pkg.routing.nodes);
      assert.equal(scan.ok, true, `${venue}: unusable anchor ${anchor.id}`);
      if (scan.ok) starts.push({ label: `sign:${anchor.id}`, id: scan.nodeId });
    }
    const offset = routeDigests.length;
    for (const profile of ['standard', 'wheelchair'] as const) {
      for (const start of starts) for (const destination of places) {
        if (start.id === destination) continue;
        const result = calculateCompiledRoute(runtime, start.id, destination, { profile });
        const id = `${venue}/${profile}/${start.label}/${destination}`;
        assert.equal(result.found, true, id);
        if (!result.found) throw new Error(id);
        assert.notEqual(result.displayClearance?.status, 'withheld', id);
        assert.equal(result.pathIds[0], start.id, id);
        assert.equal(result.pathIds.at(-1), destination, id);
        for (const [index, step] of result.steps.entries()) {
          assert.notEqual(step.type, 'u_turn', id);
          if (index) assert.notEqual(step.instruction, result.steps[index - 1].instruction, id);
          if (profile === 'wheelchair') assert.ok(!['stairs', 'escalator'].includes(step.type), id);
        }
        routeDigests.push(canonical({ id, startId: start.id, destinationId: destination, profile, sha256: sha256(canonical(result)) }));
        if (!examples[`${venue}/${profile}`]) examples[`${venue}/${profile}`] = { startId: start.id, destinationId: destination, result };
      }
    }
    const count = routeDigests.length - offset;
    assert.equal(count, expectedCounts[venueIndex], `${venue}: case inventory changed`);
    summaries.push({ venue, cases: count, packageHash: pkg.manifest.contentHash, resultsHash: sha256(routeDigests.slice(offset).join('\n') + '\n') });
  }
  return {
    'route-digests.jsonl': routeDigests.join('\n') + '\n',
    'summary.json': JSON.stringify({ formatVersion: 1, sourceCommit: PINNED_COMMIT, totalCases: routeDigests.length, venues: summaries }, null, 2) + '\n',
    'representative-routes.json': JSON.stringify(examples, null, 2) + '\n',
    'policy-cases.json': JSON.stringify(await policyCases(), null, 2) + '\n',
  };
}

export async function policyCases() {
  const pkg = await loadVenue('asterion-medical-center');
  const runtime = createCompiledBuildingRuntime(pkg);
  const closure = readJson<OperationalOverlay>('upstream/buildings/asterion-medical-center/operations/all-public-lifts-closed.overlay.json');
  const from = 'poi:poi-main-entrance', to = 'poi:poi-cardiology';
  const route = (options: CompiledRouteOptions, start = from, end = to) => calculateCompiledRoute(runtime, start, end, options);
  const cases = {
    standard: route({ profile: 'standard' }),
    stepFree: route({ profile: 'wheelchair' }),
    liftOutageStandard: route({ profile: 'standard', operationalOverlay: closure, evaluatedAt: '2026-07-22T12:00:00.000Z' }),
    liftOutageStepFree: route({ profile: 'wheelchair', operationalOverlay: closure, evaluatedAt: '2026-07-22T12:00:00.000Z' }),
    expired: route({ operationalOverlay: closure, evaluatedAt: closure.validUntil }),
    notYetActive: route({ operationalOverlay: closure, evaluatedAt: '2026-07-22T07:59:59.999Z' }),
    missingClock: route({ operationalOverlay: closure }),
    wrongPackage: route({ operationalOverlay: { ...closure, packageHash: '0'.repeat(64) }, evaluatedAt: closure.validFrom }),
    unknownClosureTarget: route({ operationalOverlay: { ...closure, closures: [{ id: 'unknown', target: { type: 'edge', id: 'missing' }, reason: 'fixture' }] }, evaluatedAt: closure.validFrom }),
    unknownStart: route({}, 'missing'),
    unknownDestination: route({}, from, 'missing'),
    sameStartAndDestination: route({}, from, from),
  };
  assert.equal(cases.stepFree.receipt.status, 'routed');
  assert.ok(cases.stepFree.receipt.selectedConnectors.every(item => item.kind !== 'stairs' && item.kind !== 'escalator'));
  assert.equal(cases.liftOutageStandard.receipt.status, 'routed');
  assert.ok(cases.liftOutageStandard.receipt.selectedConnectors.some(item => item.kind === 'stairs'));
  assert.equal(cases.liftOutageStepFree.receipt.status, 'unroutable');
  for (const key of ['expired', 'notYetActive', 'missingClock', 'wrongPackage', 'unknownClosureTarget'] as const) assert.equal(cases[key].receipt.status, 'rejected', key);
  assert.equal(cases.unknownStart.found, false);
  assert.equal(cases.unknownDestination.found, false);
  const wall = Date.parse('2026-07-22T12:00:00.000Z');
  const lease = createOperationalLease(closure, pkg, wall, 100);
  const leaseStates = [lease.read(wall, 100), lease.read(Date.parse(closure.validUntil), 100 + Date.parse(closure.validUntil) - wall), lease.read(wall, 101)];
  assert.deepEqual(leaseStates.map(state => state.status), ['current', 'unavailable', 'unavailable']);
  const tampered = structuredClone(pkg);
  tampered.building.name += ' tampered';
  await assert.rejects(() => verifyVenuePackage(tampered));
  const missingFlag = structuredClone(pkg) as CompiledBuildingPackage;
  delete (missingFlag.routing.edges[0] as Partial<typeof missingFlag.routing.edges[number]>).accessible;
  await assert.rejects(() => verifyVenuePackage(missingFlag));
  return { cases, leaseStates, rejectedPackageCases: ['content-tampering', 'missing-accessibility-flag'] };
}
