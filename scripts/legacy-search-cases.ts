import { buildGraphIndex, type GraphEdge, type GraphNode, type RouteOptions } from '../compat/voicegis/upstream/src/engine/routingCore';
import { legacySearchReference, projectLegacyGraph, projectLegacyOptions } from '../compat/voicegis/native-search';

const edge = (id: string, from: string, to: string, distance: number, extra: Partial<GraphEdge> = {}): GraphEdge => ({ id, from, to, distance, accessible: true, restricted: false, ...extra });
const node = (id: string, x: number, y: number, floor: string | number = 'g'): GraphNode => ({ id, x, y, floor, type: 'waypoint' });
export function buildLegacySearchCases() {
  const nodes = [node('a', 0, 0), node('b', 1, 1), node('c', -1, 1), node('d', 0, 2)];
  const edges = [edge('ab', 'a', 'b', 2), edge('bd', 'b', 'd', 2), edge('ac', 'a', 'c', 2), edge('cd', 'c', 'd', 2)];
  type Input = { id: string; nodes: GraphNode[]; edges: GraphEdge[]; startId?: string; destinationId?: string; options?: RouteOptions };
  const cases: Input[] = [
    { id: 'equal-priority-input-order', nodes, edges },
    { id: 'reversed-input-order', nodes, edges: [...edges].reverse() },
    { id: 'undirected-return', nodes, edges, startId: 'd', destinationId: 'a' },
    { id: 'stationary', nodes, edges, destinationId: 'a' },
    { id: 'unknown-start', nodes, edges, startId: 'missing' },
    { id: 'unknown-destination', nodes, edges, destinationId: 'missing' },
    { id: 'disconnected', nodes, edges: [] },
    { id: 'closed-first-branch', nodes, edges, options: { closedEdgeIds: ['ab'] } },
    { id: 'closed-all-branches', nodes, edges, options: { closedEdgeIds: ['ab', 'ac'] } },
    { id: 'duplicate-closure-id', nodes, edges, options: { closedEdgeIds: ['ab', 'ab'] } },
    { id: 'inaccessible-default-profile', nodes, edges: edges.map(value => value.id === 'ab' ? { ...value, accessible: false } : value) },
    { id: 'inaccessible-excluded', nodes, edges: edges.map(value => value.id === 'ab' ? { ...value, accessible: false } : value), options: { accessibleOnly: true } },
    { id: 'restricted-default-profile', nodes, edges: edges.map(value => value.id === 'ab' ? { ...value, restricted: true } : value) },
    { id: 'restricted-permitted', nodes, edges: edges.map(value => value.id === 'ab' ? { ...value, restricted: true } : value), options: { allowRestricted: true } },
    { id: 'improved-cost-obsolete-queue-entry', nodes: nodes.map(value => ({ ...value, x: 0, y: 0 })), edges: [edge('ab', 'a', 'b', 10), edge('ac', 'a', 'c', 1), edge('cb', 'c', 'b', 1), edge('bd', 'b', 'd', 20)] },
    { id: 'multi-floor-heuristic', nodes: nodes.map(value => ({ ...value, floor: value.id === 'd' ? 'first' : 'g' })), edges },
    { id: 'strict-floor-types', nodes: nodes.map(value => ({ ...value, floor: value.id === 'd' ? 0 : '0' })), edges },
    { id: 'parallel-edges', nodes, edges: [edge('short', 'a', 'd', 1), edge('long', 'a', 'd', 3)] },
    { id: 'parallel-short-edge-closed', nodes, edges: [edge('short', 'a', 'd', 1), edge('long', 'a', 'd', 3)], options: { closedEdgeIds: ['short'] } },
    { id: 'positive-self-loop', nodes, edges: [edge('self', 'a', 'a', 1), ...edges] },
  ];
  for (const [index, [x, y]] of [[0, 0], [0.1, 0.2], [1, 1], [3, 4], [1e154, 1e154], [1e-200, 1e-200]].entries()) {
    cases.push({ id: `coordinate-scale-${index}`, nodes: [node('a', 0, 0), node('d', x, y)], edges: [edge('ad', 'a', 'd', 1)] });
  }
  return cases.map(item => {
    const startId = item.startId ?? 'a', destinationId = item.destinationId ?? 'd';
    const options = projectLegacyOptions(item.options);
    return { id: item.id, synthetic: true, ...projectLegacyGraph(item.nodes, item.edges), request: { startId, destinationId, options }, heuristicScale: buildGraphIndex(item.nodes, item.edges).heuristicScale, expected: legacySearchReference(item.nodes, item.edges, startId, destinationId, options) };
  });
}
