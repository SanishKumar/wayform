import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { readFileSync } from 'node:fs';
import { canonical, loadVenue, sha256, venues } from '../compat/voicegis/harness';
import { createCompiledBuildingRuntime } from '../compat/voicegis/upstream/src/data/compiledBuilding';
import { stablePackageJson } from '../compat/voicegis/upstream/src/data/packageLifecycle';
import { createRouteGraphicGuard } from '../compat/voicegis/upstream/src/engine/routeClearance';
import { calculateRoute } from '../compat/voicegis/upstream/src/engine/routingCore';

export async function legacyClearanceCorpus() {
  const queries = readFileSync('compat/voicegis/expected/route-digests.jsonl', 'utf8').trim().split('\n').map(line => JSON.parse(line) as { id: string; startId: string; destinationId: string; profile: 'standard'|'wheelchair' });
  return Promise.all(venues.map(async venue => {
    const runtime = createCompiledBuildingRuntime(await loadVenue(venue));
    const digest = createHash('sha256'); let journeys = 0;
    for (const query of queries.filter(q => q.id.startsWith(`${venue}/`))) {
      const result = calculateRoute(query.startId, query.destinationId, runtime.routingNodes, runtime.routingEdges, { accessibleOnly: query.profile === 'wheelchair' });
      assert.ok(result.found, query.id);
      digest.update(canonical(createRouteGraphicGuard(runtime.buildingPackage, result.path, { profile: query.profile }).assessment) + '\n');
      journeys++;
    }
    return { venue, journeys, sha256: digest.digest('hex') };
  }));
}

export async function legacyClearanceNegativeCases() {
  const original = await loadVenue('asterion-medical-center');
  const runtime = createCompiledBuildingRuntime(original);
  const startId = runtime.getDefaultStartNodeId(), destinationId = 'poi:poi-cardiology';
  const route = calculateRoute(startId, destinationId, runtime.routingNodes, runtime.routingEdges, { accessibleOnly: true });
  assert.ok(route.found);
  const portalNode = route.path.find(node => node.type === 'portal')!;
  const portalIndex = original.portals.findIndex(p => p.id === portalNode.sourceId);
  const poiIndex = original.pois.findIndex(p => `poi:${p.id}` === destinationId);
  const spaceIndex = original.spaces.findIndex(s => s.id === original.pois[poiIndex].spaceId);
  const connectorNode = route.path.find(node => node.type === 'connector-stop')!;
  const connectorIndex = original.verticalConnectors.findIndex(c => c.id === connectorNode.sourceId);
  const stopIndex = original.verticalConnectors[connectorIndex].stops.findIndex(s => s.floorId === connectorNode.floor);
  type Mutation = { id: string; path: (string|number)[]; value: unknown };
  const mutations: Mutation[] = [
    { id: 'destination-excluded', path: ['pois', poiIndex, 'accessible'], value: false },
    { id: 'room-restricted', path: ['spaces', spaceIndex, 'public'], value: false },
    { id: 'room-excluded', path: ['spaces', spaceIndex, 'accessible'], value: false },
    { id: 'room-outside', path: ['spaces', spaceIndex, 'polygon'], value: [[1000,1000],[1010,1000],[1010,1010],[1000,1010]] },
    { id: 'narrow-door', path: ['portals', portalIndex, 'width'], value: 0.1 },
    { id: 'opening-beyond-wall', path: ['portals', portalIndex, 'width'], value: 10000 },
    { id: 'portal-restricted', path: ['portals', portalIndex, 'restricted'], value: true },
    { id: 'portal-excluded', path: ['portals', portalIndex, 'accessible'], value: false },
    { id: 'portal-shifted', path: ['portals', portalIndex, 'position', 0], value: original.portals[portalIndex].position[0] + 0.2 },
    { id: 'connector-shifted', path: ['verticalConnectors', connectorIndex, 'stops', stopIndex, 'position', 0], value: original.verticalConnectors[connectorIndex].stops[stopIndex].position[0] + 0.2 },
    { id: 'connector-restricted', path: ['verticalConnectors', connectorIndex, 'restricted'], value: true },
  ];
  return mutations.map(mutation => {
    const pkg = structuredClone(original);
    let target: any = pkg;
    for (const key of mutation.path.slice(0,-1)) target = target[key];
    target[mutation.path.at(-1)!] = mutation.value;
    const { manifest: _, ...content } = pkg;
    pkg.manifest.contentHash = sha256(stablePackageJson(content));
    const expected = createRouteGraphicGuard(pkg, route.path, { profile: 'wheelchair' }).assessment;
    assert.equal(expected.status, 'withheld', mutation.id);
    return { ...mutation, venue: 'asterion-medical-center', startId, destinationId, expected };
  });
}
