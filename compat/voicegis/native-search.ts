import assert from 'node:assert/strict';
import type { CompiledBuildingPackage } from './upstream/packages/map-compiler/src/compiler';
import { calculateRoute, type GraphEdge, type GraphNode, type RouteOptions, type RouteResult } from './upstream/src/engine/routingCore';

export function projectLegacyPolicyPackage(pkg: CompiledBuildingPackage) {
  return { buildingId: pkg.building.id, packageHash: pkg.manifest.contentHash, edges: pkg.routing.edges.map(({ id, sourceId }) => ({ id, sourceId })) };
}

export function projectLegacyGraph(nodes: GraphNode[], edges: GraphEdge[]) {
  return {
    nodes: nodes.map(({ id, x, y, floor }) => ({ id, x, y, floor })),
    edges: edges.map(({ id, from, to, distance, accessible, restricted }) => {
      assert.equal(typeof id, 'string'); assert.equal(typeof accessible, 'boolean'); assert.equal(typeof restricted, 'boolean');
      return { id: id!, from, to, distance, accessible: accessible!, restricted: restricted! };
    }),
  };
}

export function projectLegacyOptions(options: RouteOptions = {}) {
  return { accessibleOnly: options.accessibleOnly ?? false, allowRestricted: options.allowRestricted ?? false, closedEdgeIds: [...options.closedEdgeIds ?? []] };
}

export function projectLegacyResult(result: RouteResult) {
  if (result.found) return { outcome: 'routed', pathIds: result.pathIds, totalDistance: result.totalDistance };
  if (result.error === 'Start or destination does not exist in the building graph.') return { outcome: 'unknown-endpoint' };
  assert.equal(result.error, 'No route satisfies the selected constraints.');
  return { outcome: 'no-route' };
}

export function legacySearchReference(nodes: GraphNode[], edges: GraphEdge[], startId: string, destinationId: string, options: RouteOptions) {
  return projectLegacyResult(calculateRoute(startId, destinationId, nodes, edges, options));
}
