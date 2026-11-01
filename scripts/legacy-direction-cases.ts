import { createHash } from 'node:crypto';
import { readFileSync } from 'node:fs';
import { canonical, loadVenue, venues } from '../compat/voicegis/harness';
import { createCompiledBuildingRuntime } from '../compat/voicegis/upstream/src/data/compiledBuilding';
import { calculateRoute, type RouteStep } from '../compat/voicegis/upstream/src/engine/routingCore';
import { describeWithLandmarks, landmarksFrom } from '../compat/voicegis/upstream/src/engine/routeLandmarks';

// Platform atan2 can differ by a few ulps. Only bearings are rounded for the
// aggregate comparison; wording, distances, IDs and path metadata remain exact.
export function comparableSteps(steps: RouteStep[]) {
  return steps.map(step => ({ ...step, bearing: Math.round(step.bearing * 1e9) / 1e9 }));
}
export async function legacyDirectionCases(withLandmarks = false) {
  const queries = readFileSync('compat/voicegis/expected/route-digests.jsonl', 'utf8').trim().split('\n').map(line => JSON.parse(line) as { id: string; startId: string; destinationId: string; profile: string });
  return Promise.all(venues.map(async venue => {
    const runtime = createCompiledBuildingRuntime(await loadVenue(venue));
    const digest = createHash('sha256');
    let journeys = 0;
    for (const query of queries.filter(q => q.id.startsWith(`${venue}/`))) {
      const result = calculateRoute(query.startId, query.destinationId, runtime.routingNodes, runtime.routingEdges, { accessibleOnly: query.profile === 'wheelchair' });
      if (!result.found) throw new Error(`Unexpected missing fixture path: ${query.id}`);
      const steps = withLandmarks ? describeWithLandmarks(result.steps, result.path, landmarksFrom(runtime.buildingPackage)) : result.steps;
      const value = JSON.parse(JSON.stringify({ path: result.path, steps: comparableSteps(steps) }));
      digest.update(canonical(value) + '\n');
      journeys++;
    }
    return { venue, journeys, sha256: digest.digest('hex') };
  }));
}
