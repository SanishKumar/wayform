import { CONTRACT_VERSION, type EdgeKind, type MobilityBundle } from '../../packages/mobility-contract/src/types';
import { validateBundle } from '../../packages/mobility-contract/src/validate';
import { loadVenue, PINNED_COMMIT, venues, verifySourceSnapshot } from './harness';

/** Explicit topology adapter, not an assertion that the fixture was surveyed. */
export async function adaptPinnedVenue(venue: typeof venues[number]): Promise<MobilityBundle> {
  verifySourceSnapshot();
  const pkg = await loadVenue(venue);
  const frameId = `${pkg.building.id}:plan`;
  const floors = new Map(pkg.floors.map(floor => [floor.id, floor.elevation]));
  const bundle: MobilityBundle = {
    schemaVersion: CONTRACT_VERSION,
    id: pkg.building.id,
    revision: pkg.manifest.contentHash,
    frames: [{ id: frameId, kind: 'local', axes: 'x-right-y-down-z-up' }],
    nodes: pkg.routing.nodes.map(node => ({ id: node.id, frameId, position: [node.position[0], node.position[1], floors.get(node.floorId)!], floorId: node.floorId })),
    edges: pkg.routing.edges.flatMap(edge => {
      const kind: EdgeKind = edge.connectorKind === 'elevator' ? 'lift'
        : edge.connectorKind === 'stairs' || edge.connectorKind === 'escalator' ? 'stairs'
          : edge.connectorKind === 'ramp' ? 'ramp' : 'walkway';
      const base = { lengthMeters: edge.distanceMeters, kind, access: edge.restricted ? 'restricted' as const : 'public' as const, transformId: null,
        legacy: { accessible: edge.accessible, meaning: 'author-assertion' as const, sourceCommit: PINNED_COMMIT, packageHash: pkg.manifest.contentHash, sourceEdgeId: edge.id } };
      return [
        { ...base, id: `${edge.id}:forward`, from: edge.from, to: edge.to },
        { ...base, id: `${edge.id}:reverse`, from: edge.to, to: edge.from },
      ];
    }),
    // A portal's authored width and an accessible flag are not surveyed evidence.
    evidence: [], observations: [], transforms: [], policyRequirements: [],
  };
  const issues = validateBundle(bundle);
  if (issues.length) throw new Error(issues.join('\n'));
  return bundle;
}
