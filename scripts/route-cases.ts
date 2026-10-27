import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { assessPath } from '../packages/mobility-contract/src/assess';
import { inputDigests } from '../packages/mobility-contract/src/identity';
import type { EvaluationContext, MobilityBundle, MobilityProfile, PathAssessment } from '../packages/mobility-contract/src/types';
import { validateBundle, validateContext, validateProfile } from '../packages/mobility-contract/src/validate';
import { addEntrance, addObservation, addPolicy, fixture } from '../tests/fixtures';

type Data = { bundle: MobilityBundle; profile: MobilityProfile; context: EvaluationContext };
type Outcome = 'routed' | 'insufficient-evidence' | 'infeasible' | 'rejected';
type Scenario = Data & { id: string; startId: string; destinationId: string; outcome: Outcome; edgeIds?: string[]; errorCode?: string };

function detour(): Data {
  const data = fixture();
  data.bundle.edges.push({ ...data.bundle.edges[0], id: 'ac', to: 'c', lengthMeters: 20 }, { ...data.bundle.edges[0], id: 'cb', from: 'c', lengthMeters: 20 });
  addObservation(data.bundle, 'clearWidthMeters', 1, 0.02, 'ac'); addObservation(data.bundle, 'clearWidthMeters', 1, 0.02, 'cb');
  return data;
}

export function buildRouteCases() {
  const document = JSON.parse(readFileSync(new URL('../packages/mobility-contract/examples/decisions-v0.1.0-draft.1.json', import.meta.url), 'utf8')) as {
    examples: (Data & { id: string; request: { startId: string; destinationId: string; edgeIds: string[] }; expected: PathAssessment })[];
  };
  const scenarios: Scenario[] = document.examples.map(item => ({ ...item, ...item.request, outcome: item.expected.status === 'suitable' ? 'routed' : item.expected.status === 'unknown' ? 'insufficient-evidence' : 'infeasible' }));
  for (const item of [
    { id: 'suitable-shortcut', change(_data: Data) {}, edgeIds: ['ab'] },
    { id: 'narrow-shortcut', change(data: Data) { data.bundle.observations[0].value = 0.6; }, edgeIds: ['ac', 'cb'] },
    { id: 'unknown-shortcut', change(data: Data) { data.bundle.observations.shift(); }, edgeIds: ['ac', 'cb'] },
    { id: 'closed-shortcut', change(data: Data) { addPolicy(data.bundle, data.context); data.context.policies[0].closedEdgeIds = ['ab']; }, edgeIds: ['ac', 'cb'] },
    { id: 'preferred-detour', change(data: Data) { data.bundle.edges[0].kind = 'lift'; data.profile.preferences = { avoidKinds: ['lift'], penaltyMeters: 50 }; }, edgeIds: ['ac', 'cb'] },
    { id: 'stairs-excluded', change(data: Data) { data.bundle.edges[0].kind = 'stairs'; data.profile.hard.maxStepCount = 0; for (const edge of data.bundle.edges) addObservation(data.bundle, 'stepCount', 0, 0, edge.id); }, edgeIds: ['ac', 'cb'] },
  ]) {
    const data = detour(); item.change(data);
    scenarios.push({ ...data, id: item.id, startId: 'a', destinationId: 'b', outcome: 'routed', edgeIds: item.edgeIds });
  }
  const extra: { id: string; change?(data: Data): void; start?: string; end?: string; outcome: Outcome; edgeIds?: string[]; errorCode?: string }[] = [
    { id: 'stationary', end: 'a', outcome: 'routed', edgeIds: [] },
    { id: 'stationary-with-unknown-edges', end: 'a', change(data) { data.bundle.observations = []; }, outcome: 'routed', edgeIds: [] },
    { id: 'disconnected', end: 'c', outcome: 'infeasible' },
    { id: 'unknown-start', start: 'missing', outcome: 'rejected', errorCode: 'invalid-search' },
    { id: 'unknown-destination', end: 'missing', outcome: 'rejected', errorCode: 'invalid-search' },
    { id: 'registered-entrance', change(data) { addEntrance(data.bundle); }, outcome: 'routed', edgeIds: ['ab'] },
    { id: 'unbound-policy-after-edit', change(data) { addPolicy(data.bundle, data.context); data.bundle.nodes[0].position[2] = 1; }, outcome: 'insufficient-evidence' },
    { id: 'policy-expired-and-closed', change(data) { addPolicy(data.bundle, data.context); data.context.policies[0].closedEdgeIds = ['ab']; data.context.evaluatedAt = data.context.policies[0].validUntil; }, outcome: 'insufficient-evidence' },
    { id: 'parallel-utf16-tie', change(data) {
      data.bundle.edges[0].id = '\ue000'; data.bundle.observations[0].edgeId = '\ue000';
      data.bundle.edges.push({ ...data.bundle.edges[0], id: '\u{10000}' }); addObservation(data.bundle, 'clearWidthMeters', 1, 0.02, '\u{10000}');
    }, outcome: 'routed', edgeIds: ['\u{10000}'] },
    { id: 'unknown-and-closed-frontier', change(data) {
      data.bundle.observations = [];
      data.bundle.edges.push({ ...data.bundle.edges[0], id: 'blocked', to: 'c', access: 'restricted' });
    }, outcome: 'insufficient-evidence' },
    { id: 'unreachable-reasons-excluded', change(data) {
      data.bundle.observations = [];
      data.bundle.edges.push({ ...data.bundle.edges[0], id: 'unreachable', from: 'c', access: 'restricted' });
    }, outcome: 'insufficient-evidence' },
    { id: 'search-cost-overflow', change(data) { data.bundle.edges[0].lengthMeters = Number.MAX_VALUE; data.profile.preferences = { avoidKinds: ['walkway'], penaltyMeters: Number.MAX_VALUE }; }, outcome: 'rejected', errorCode: 'unrepresentable-cost' },
    { id: 'search-nonadvancing-cost', end: 'c', change(data) {
      data.bundle.edges[0].lengthMeters = 2 ** 54;
      data.bundle.edges.push({ ...data.bundle.edges[0], id: 'bc', from: 'b', to: 'c', lengthMeters: 1 }); addObservation(data.bundle, 'clearWidthMeters', 1, 0.02, 'bc');
    }, outcome: 'rejected', errorCode: 'unrepresentable-cost' },
  ];
  for (const item of extra) {
    const data = fixture(); item.change?.(data);
    scenarios.push({ ...data, id: item.id, startId: item.start ?? 'a', destinationId: item.end ?? 'b', outcome: item.outcome, edgeIds: item.edgeIds, errorCode: item.errorCode });
  }
  return scenarios.map(item => {
    const { bundle, profile, context, startId, destinationId } = item;
    assert.deepEqual(validateBundle(bundle), [], item.id); assert.deepEqual(validateProfile(profile), [], item.id); assert.deepEqual(validateContext(context, bundle), [], item.id);
    const snapshot = inputDigests(bundle, profile, context);
    let expected: object;
    if (item.outcome === 'rejected') expected = { outcome: item.outcome, snapshot, errorCode: item.errorCode };
    else if (item.outcome === 'routed') {
      const assessment = assessPath(bundle, profile, context, startId, destinationId, item.edgeIds!);
      assert.equal(assessment.status, 'suitable', item.id);
      expected = { outcome: item.outcome, snapshot, assessment };
    } else {
      const decisions = new Map(bundle.edges.map(edge => [edge.id, assessPath(bundle, profile, context, edge.from, edge.to, [edge.id])]));
      const reach = (allowUnknown: boolean) => {
        const reached = new Set([startId]);
        for (const node of reached) for (const edge of bundle.edges) {
          const status = decisions.get(edge.id)!.status;
          if (edge.from === node && (status === 'suitable' || (allowUnknown && status === 'unknown'))) reached.add(edge.to);
        }
        return reached;
      };
      const reached = reach(false);
      assert.equal(reached.has(destinationId), false, item.id);
      assert.equal(reach(true).has(destinationId), item.outcome === 'insufficient-evidence', item.id);
      const frontier = bundle.edges.filter(edge => reached.has(edge.from) && !reached.has(edge.to)).map(edge => decisions.get(edge.id)!);
      expected = { outcome: item.outcome, snapshot, diagnostic: {
        suitableReachableNodes: reached.size,
        unknownFrontierEdges: frontier.filter(value => value.status === 'unknown').length,
        unsuitableFrontierEdges: frontier.filter(value => value.status === 'unsuitable').length,
        reasonCodes: [...new Set(frontier.flatMap(value => value.reasons.map(reason => reason.code)))].sort(),
      } };
    }
    return { id: item.id, synthetic: true, bundle, profile, context, request: { startId, destinationId }, expected };
  });
}
