import { canonical, loadVenue, sha256, venues } from '../compat/voicegis/harness';
import { createCompiledBuildingRuntime } from '../compat/voicegis/upstream/src/data/compiledBuilding';

export async function legacyRuntimeCases() {
  return Promise.all(venues.map(async venue => {
    const runtime = createCompiledBuildingRuntime(await loadVenue(venue));
    const projection = JSON.parse(JSON.stringify({ nodes: runtime.routingNodes, edges: runtime.routingEdges, defaultStartNodeId: runtime.getDefaultStartNodeId() }));
    return { venue, nodes: runtime.routingNodes.length, edges: runtime.routingEdges.length,
      publicDestinations: runtime.getPOIs().map(node => node.id), sha256: sha256(canonical(projection)) };
  }));
}
