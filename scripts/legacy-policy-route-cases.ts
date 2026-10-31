import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { PINNED_COMMIT, loadVenue } from '../compat/voicegis/harness';
import { legacySearchReference, projectLegacyGraph, projectLegacyPolicyPackage } from '../compat/voicegis/native-search';
import { createCompiledBuildingRuntime } from '../compat/voicegis/upstream/src/data/compiledBuilding';
import type { GraphNode, GraphEdge } from '../compat/voicegis/upstream/src/engine/routingCore';
import type { OverlayResolution } from '../compat/voicegis/upstream/src/engine/operationalOverlay';
import { buildLegacyOverlayCases } from './legacy-overlay-cases';

export type PolicyQuery = {
  queryId: string; startId: string; destinationId: string;
  options: { accessibleOnly: boolean; allowRestricted: boolean };
  policy: { mode: 'none' } | { mode: 'overlay'; overlay: unknown; evaluatedAt: string };
};
export type PolicyRequest = ReturnType<typeof projectLegacyGraph> & {
  protocolVersion: '0.2.0'; sourceCommit: string; requestId: string;
  package: ReturnType<typeof projectLegacyPolicyPackage>; queries: PolicyQuery[];
};
export type PolicyExpected = {
  queryId: string; outcome: string; pathIds?: string[]; totalDistance?: number;
  overlayResolution?: OverlayResolution; error?: { code: string; message: string };
};
export type PolicyBatch = { request: PolicyRequest; expected: PolicyExpected[] };

export async function buildLegacyPolicyRouteCases(): Promise<PolicyBatch[]> {
  const corpus = await buildLegacyOverlayCases();
  assert.deepEqual(corpus, JSON.parse(readFileSync(new URL('../compat/voicegis/expected/operational-overlays.json', import.meta.url), 'utf8')));
  const batches: PolicyBatch[] = [];
  for (const group of corpus.groups) {
    let nodes: GraphNode[], edges: GraphEdge[], startId: string, destinationId: string;
    if (group.id === 'synthetic') {
      nodes = ['a', 'b', 'd'].map(id => ({ id, x: 0, y: 0, floor: 'ground', type: 'waypoint' }));
      edges = [
        { id: 'door-edge', from: 'a', to: 'b', distance: 1, accessible: false, restricted: false },
        { id: 'edge-\u{10000}', from: 'b', to: 'd', distance: 1, accessible: true, restricted: false },
        { id: 'edge-\uE000', from: 'a', to: 'd', distance: 5, accessible: true, restricted: false },
      ];
      startId = 'a'; destinationId = 'd';
    } else {
      const pkg = await loadVenue(group.id);
      assert.deepEqual(projectLegacyPolicyPackage(pkg), group.package);
      const runtime = createCompiledBuildingRuntime(pkg);
      nodes = runtime.routingNodes; edges = runtime.routingEdges;
      startId = 'poi:poi-main-entrance'; destinationId = 'poi:poi-cardiology';
    }
    const batch: PolicyBatch = { request: {
      protocolVersion: '0.2.0', sourceCommit: PINNED_COMMIT, requestId: `policy/${group.id}`,
      ...projectLegacyGraph(nodes, edges), package: group.package, queries: [],
    }, expected: [] };
    const add = (query: PolicyQuery, resolution?: OverlayResolution) => {
      batch.request.queries.push(query);
      const result = resolution && !resolution.valid
        ? { outcome: 'rejected', error: { code: 'invalid-overlay', message: 'operational overlay rejected; search was not run' } }
        : legacySearchReference(nodes, edges, query.startId, query.destinationId, { ...query.options, closedEdgeIds: resolution?.closedEdgeIds ?? [] });
      batch.expected.push({ queryId: query.queryId, ...result, ...(resolution ? { overlayResolution: resolution } : {}) });
    };
    for (const accessibleOnly of [false, true]) {
      add({ queryId: `none/${accessibleOnly}`, startId, destinationId, options: { accessibleOnly, allowRestricted: false }, policy: { mode: 'none' } });
    }
    for (const item of group.cases) for (const accessibleOnly of [false, true]) {
      add({ queryId: `${item.id}/${accessibleOnly}`, startId, destinationId, options: { accessibleOnly, allowRestricted: false }, policy: { mode: 'overlay', overlay: item.overlay, evaluatedAt: item.evaluatedAt } }, item.expected);
    }
    const expired = group.cases.find(item => item.expected.issues.some(issue => issue.code === 'overlay-expired'))!;
    for (const [id, destination] of [['stationary-expired', startId], ['unknown-endpoint-expired', 'missing']]) {
      add({ queryId: id, startId, destinationId: destination, options: { accessibleOnly: false, allowRestricted: false }, policy: { mode: 'overlay', overlay: expired.overlay, evaluatedAt: expired.evaluatedAt } }, expired.expected);
    }
    add({ queryId: 'none-after-rejections', startId, destinationId, options: { accessibleOnly: false, allowRestricted: false }, policy: { mode: 'none' } });
    batches.push(batch);
  }
  assert.equal(batches.reduce((sum, batch) => sum + batch.request.queries.length, 0), 126);
  return batches;
}
