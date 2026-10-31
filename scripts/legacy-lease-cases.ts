import { PINNED_COMMIT, loadVenue } from '../compat/voicegis/harness';
import type { CompiledBuildingPackage } from '../compat/voicegis/upstream/packages/map-compiler/src/compiler';
import { copyOperationalOverlay, createOperationalLease } from '../compat/voicegis/upstream/src/navigation/operationalLease';
import { buildLegacyOverlayCases } from './legacy-overlay-cases';

export type Clock = number | 'NaN' | 'Infinity' | '-Infinity';
export type LeaseEvent = { operation: 'read' | 'delay'; wallMs: Clock; monotonicMs: Clock };
type Input = { id: string; overlay: unknown; initial: { wallMs: Clock; monotonicMs: Clock }; events: LeaseEvent[] };
export const decodeClock = (value: Clock): number => typeof value === 'number' ? value : Number(value);
const encodeClock = (value: number): Clock => Number.isFinite(value) ? value : String(value) as Exclude<Clock, number>;

export async function buildLegacyLeaseCases() {
  const overlays = await buildLegacyOverlayCases();
  const now = Date.parse('2026-07-22T12:00:00.000Z');
  const mono = 100;
  const base = {
    schemaVersion: '0.1.0', id: 'short-outage', buildingId: 'fixture', packageHash: 'a'.repeat(64),
    validFrom: new Date(now - 1000).toISOString(), validUntil: new Date(now + 5000).toISOString(),
    closures: [{ id: 'close-lift', target: { type: 'source', id: 'lift' }, reason: 'Synthetic outage' }],
  };
  const read = (wall: number, monotonic = wall): LeaseEvent => ({ operation: 'read', wallMs: now + wall, monotonicMs: mono + monotonic });
  const delay = (wall: number, monotonic = wall): LeaseEvent => ({ ...read(wall, monotonic), operation: 'delay' });
  const cases: Input[] = [];
  const add = (id: string, events: LeaseEvent[], overlay: unknown = base, initial = { wallMs: now as Clock, monotonicMs: mono as Clock }) => {
    cases.push({ id, overlay, initial, events });
  };
  add('wall-expiry-stays-unavailable', [read(0), delay(4500), read(5000), read(4500, 5100), read(5100)]);
  add('monotonic-expiry-with-frozen-wall', [read(0), read(0, 4999), delay(0, 4999.75), read(0, 5000), read(1, 5001)]);
  add('wall-jump-expires-before-monotonic', [read(0), read(5000, 1), read(4500, 2)]);
  add('both-boundaries-exclusive', [read(4999.75), delay(4999.75), read(5000), delay(5000)]);
  add('rollback-exactly-one-second', [read(0), read(-1000, 1), read(0, 2)]);
  add('rollback-over-one-second-latches', [read(0), read(-1000.25, 1), read(100, 2)]);
  add('small-rollbacks-use-wall-high-water', [read(0), read(-500, 1), read(-900, 2), read(-1001, 3), read(100, 4)]);
  add('advanced-wall-high-water', [read(300), read(-700, 301), read(-701, 302), read(400, 400)]);
  add('monotonic-rollback-latches', [read(0), read(100, -1), read(200, 200)]);
  add('fractional-monotonic-rollback', [read(0), read(0, -0.001), read(100)]);
  add('repeated-monotonic-tick', [read(0), read(1, 0), read(2, 0)]);
  add('fractional-clock-and-delay', [read(0.25, 0.5), delay(4499.5, 4499.25), delay(4999.75, 4999.5), read(5000.25, 5000)]);
  add('delay-does-not-observe-expiry', [delay(6000), read(0), delay(0), read(5000)]);
  add('delay-does-not-change-high-water', [delay(10_000), read(0), delay(-10_000, -10_000), read(100)]);
  add('delay-clamps-and-prefers-earlier-clock', [delay(0), delay(4000), delay(4500, 4600), delay(4999.75), delay(5000)]);
  for (const invalid of ['NaN', 'Infinity', '-Infinity'] as const) {
    for (const field of ['wallMs', 'monotonicMs'] as const) {
      const broken = { ...read(0), [field]: invalid };
      add(`nonfinite-read-${field}-${invalid}`, [read(0), broken, read(100), delay(100)]);
      add(`nonfinite-initial-${field}-${invalid}`, [read(0), read(100), delay(100)], base, { wallMs: now, monotonicMs: mono, [field]: invalid });
      add(`nonfinite-delay-${field}-${invalid}`, [{ ...delay(4750), [field]: invalid }, read(0)]);
    }
  }
  const future = { ...base, validFrom: new Date(now + 1000).toISOString() };
  add('future-policy-activates-on-read', [read(0), read(999), read(1000), read(1100), read(5000)], future);
  add('first-read-after-start', [delay(0), read(1200), read(5000)], future);
  add('future-policy-skips-valid-window', [read(0), read(5000), read(4500, 5100)], future);
  add('active-policy-survives-tolerated-start-rollback', [read(0), read(1000), read(500, 1001), read(5000)], future);
  add('future-policy-other-issue-remains-blocked', [read(0), read(1000), read(5000)], { ...future, schemaVersion: 'future' });
  add('clock-error-dominates-expiry', [read(0), read(5000), read(0, 5100), read(6000)], base);
  add('expiry-dominates-overlay-error', [read(0), read(5000)], { ...base, packageHash: 'b'.repeat(64) });
  add('malformed-shape-does-not-create-deadline', [read(0), read(5000)], { ...base, closures: null });
  add('future-then-clock-loss-stays-blocked', [read(0), read(-1, -1), read(1000), read(1100)], future);
  for (const text of ['0000-01-01T00:00:00.000Z', '1969-12-31T23:59:59.998Z', '2000-02-28T23:59:59.999Z', '9999-12-31T23:59:59.997Z']) {
    const wall = Date.parse(text);
    const short = { ...base, validFrom: text, validUntil: new Date(wall + 2).toISOString() };
    add(`calendar-${text}`, [
      { operation: 'read', wallMs: wall, monotonicMs: 0 },
      { operation: 'read', wallMs: wall + 1, monotonicMs: 1 },
      { operation: 'read', wallMs: wall + 2, monotonicMs: 2 },
    ], short, { wallMs: wall, monotonicMs: 0 });
  }
  const zeroOverlay = { ...base, validFrom: '1969-12-31T23:59:59.999Z', validUntil: '1970-01-01T00:00:00.002Z' };
  add('negative-fractional-wall-truncation', [
    { operation: 'read', wallMs: -0.5, monotonicMs: 0 },
    { operation: 'read', wallMs: 0.5, monotonicMs: 1 },
    { operation: 'delay', wallMs: 1.5, monotonicMs: 2 },
    { operation: 'read', wallMs: 2, monotonicMs: 2.5 },
  ], zeroOverlay, { wallMs: -0.5, monotonicMs: 0 });

  const groups = [];
  for (const group of overlays.groups) {
    const pkg = group.id === 'synthetic'
      ? { building: { id: group.package.buildingId }, manifest: { contentHash: group.package.packageHash }, routing: { edges: group.package.edges } } as unknown as CompiledBuildingPackage
      : await loadVenue(group.id);
    const inputs: Input[] = group.id === 'synthetic' ? [...cases] : [];
    for (const item of group.cases) {
      const wall = Date.parse(item.evaluatedAt);
      inputs.push({ id: `overlay/${item.id}`, overlay: item.overlay, initial: { wallMs: encodeClock(wall), monotonicMs: mono }, events: [
        { operation: 'read', wallMs: encodeClock(wall), monotonicMs: mono },
        { operation: 'delay', wallMs: encodeClock(wall), monotonicMs: mono },
        { operation: 'read', wallMs: encodeClock(wall + 1), monotonicMs: mono + 1 },
      ] });
    }
    groups.push({ id: group.id, package: group.package, cases: inputs.map(input => {
      const lease = createOperationalLease(copyOperationalOverlay(structuredClone(input.overlay)), pkg, decodeClock(input.initial.wallMs), decodeClock(input.initial.monotonicMs));
      return { ...input, expected: input.events.map(event => lease[event.operation](decodeClock(event.wallMs), decodeClock(event.monotonicMs))) };
    }) });
  }
  return { formatVersion: 1, sourceCommit: PINNED_COMMIT, groups };
}
