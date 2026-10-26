import assert from 'node:assert/strict';
import { assessPath } from '../packages/mobility-contract/src/assess';
import type { Attribute, PathAssessment } from '../packages/mobility-contract/src/types';
import { addObservation, fixture } from '../tests/fixtures';

type Data = ReturnType<typeof fixture>;
type Status = PathAssessment['status'];

/** Isolate one physical requirement; access, policy and transform tests live elsewhere. */
function measurement(attribute: Attribute): Data {
  const data = fixture();
  data.profile.hard = { minClearWidthMeters: null, maxUphillPercent: null, maxDownhillPercent: null, maxCrossSlopePercent: null, maxStepCount: null, allowedSurfaces: null, allowedDoorOperations: null };
  data.bundle.observations = [];
  switch (attribute) {
    case 'clearWidthMeters': data.profile.hard.minClearWidthMeters = 0.8; addObservation(data.bundle, attribute, 1, 0.02); break;
    case 'slopePercent': data.profile.hard.maxUphillPercent = 5; data.profile.hard.maxDownhillPercent = 6; addObservation(data.bundle, attribute, 3, 0.2); break;
    case 'crossSlopePercent': data.profile.hard.maxCrossSlopePercent = 2; addObservation(data.bundle, attribute, 1, 0.1); break;
    case 'stepCount': data.profile.hard.maxStepCount = 0; addObservation(data.bundle, attribute, 0, 0); break;
    case 'surface': data.profile.hard.allowedSurfaces = ['paved']; addObservation(data.bundle, attribute, 'paved', null); break;
    case 'doorOperation': data.profile.hard.allowedDoorOperations = ['automatic']; data.bundle.edges[0].kind = 'door'; addObservation(data.bundle, attribute, 'automatic', null); break;
  }
  return data;
}

export function buildMeasurementCases() {
  const cases: { id: string; attribute: Attribute; data: Data; status: Status }[] = [];
  const add = (id: string, attribute: Attribute, status: Status, change: (data: Data) => void = () => {}) => {
    const data = measurement(attribute); change(data); cases.push({ id, attribute, data, status });
  };
  const failures: Record<Attribute, number | string> = { clearWidthMeters: 0.6, slopePercent: 8, crossSlopePercent: 3, stepCount: 2, surface: 'gravel', doorOperation: 'manual' };
  for (const attribute of Object.keys(failures) as Attribute[]) {
    add(`${attribute}/suitable`, attribute, 'suitable');
    add(`${attribute}/missing`, attribute, 'unknown', data => { data.bundle.observations = []; });
    add(`${attribute}/explicit-unknown`, attribute, 'unknown', data => { Object.assign(data.bundle.observations[0], { value: null, uncertainty: null, unknownReason: 'not-surveyed' }); });
    add(`${attribute}/excluded`, attribute, 'unsuitable', data => { data.bundle.observations[0].value = failures[attribute]; });
    add(`${attribute}/conflicting`, attribute, 'unknown', data => { addObservation(data.bundle, attribute, failures[attribute], data.bundle.observations[0].uncertainty); });
    if (typeof failures[attribute] === 'number') add(`${attribute}/missing-error`, attribute, 'unknown', data => { data.bundle.observations[0].uncertainty = null; });
  }
  const width = (id: string, status: Status, change: (data: Data) => void) => add(`width/${id}`, 'clearWidthMeters', status, change);
  width('threshold', 'suitable', data => { Object.assign(data.bundle.observations[0], { value: 0.8, uncertainty: 0 }); });
  width('arithmetic-noise', 'suitable', data => { Object.assign(data.bundle.observations[0], { value: 0.82, uncertainty: 0.02 }); });
  width('real-straddle', 'unknown', data => { Object.assign(data.bundle.observations[0], { value: 0.819999999996, uncertainty: 0.02 }); });
  width('overlap-envelope', 'unknown', data => { Object.assign(data.bundle.observations[0], { value: 0.85, uncertainty: 0.1 }); addObservation(data.bundle, 'clearWidthMeters', 0.9, 0.01); });
  width('overflow-interval', 'unknown', data => { Object.assign(data.bundle.observations[0], { value: Number.MAX_VALUE, uncertainty: Number.MAX_VALUE }); });
  width('age-boundary', 'suitable', data => { data.profile.evidencePolicy.maxAgeSeconds = 7200; });
  width('stale', 'unknown', data => { data.profile.evidencePolicy.maxAgeSeconds = 7199.999; });
  width('expiry', 'unknown', data => { data.bundle.evidence[0].validUntil = data.context.evaluatedAt; });
  width('future-observation', 'unknown', data => { data.bundle.evidence[0].observedAt = '2026-10-25T13:00:00.000Z'; data.bundle.evidence[0].review.reviewedAt = '2026-10-25T13:15:00.000Z'; });
  width('future-review', 'unknown', data => { data.bundle.evidence[0].review.reviewedAt = '2026-10-25T12:00:00.001Z'; });
  width('pending-review', 'unknown', data => { data.bundle.evidence[0].review = { status: 'pending', reviewerId: null, reviewedAt: null }; });
  width('optional-review', 'suitable', data => { data.profile.evidencePolicy.requireReviewed = false; data.bundle.evidence[0].review = { status: 'pending', reviewerId: null, reviewedAt: null }; });
  width('rejected-review', 'unknown', data => { data.profile.evidencePolicy.requireReviewed = false; data.bundle.evidence[0].review.status = 'rejected'; });
  width('unaccepted-source', 'unknown', data => { data.bundle.evidence[0].source = 'inference'; });
  width('source-before-review-and-age', 'unknown', data => { data.bundle.evidence[0].source = 'inference'; data.bundle.evidence[0].review.status = 'rejected'; data.profile.evidencePolicy.maxAgeSeconds = 0; });
  width('stale-conflict-ignored', 'suitable', data => {
    data.bundle.evidence.push({ ...data.bundle.evidence[0], id: 'old', validUntil: '2026-10-25T11:00:00.000Z' });
    addObservation(data.bundle, 'clearWidthMeters', 0.6, 0.02); data.bundle.observations[1].evidenceId = 'old';
  });
  width('fresh-newer-conflict', 'unknown', data => {
    data.bundle.evidence.push({ ...data.bundle.evidence[0], id: 'newer', observedAt: '2026-10-25T11:00:00.000Z', review: { ...data.bundle.evidence[0].review, reviewedAt: '2026-10-25T11:15:00.000Z' } });
    addObservation(data.bundle, 'clearWidthMeters', 0.6, 0.02); data.bundle.observations[1].evidenceId = 'newer';
  });
  width('unicode-provenance-order', 'suitable', data => {
    data.bundle.evidence[0].id = '\uE000'; data.bundle.observations[0].evidenceId = '\uE000';
    data.bundle.evidence.push({ ...data.bundle.evidence[0], id: '\u{10000}' });
    addObservation(data.bundle, 'clearWidthMeters', 1, 0.03); data.bundle.observations[1].evidenceId = '\u{10000}';
  });
  width('multiple-unavailable-reasons', 'unknown', data => {
    data.bundle.evidence[0].validUntil = data.context.evaluatedAt;
    data.bundle.evidence.push({ ...data.bundle.evidence[0], id: 'pending', validUntil: null, review: { status: 'pending', reviewerId: null, reviewedAt: null } });
    addObservation(data.bundle, 'clearWidthMeters', null, null); data.bundle.observations[1].evidenceId = 'pending';
  });
  for (const time of ['0000-01-01T00:00:00.000Z', '9999-12-31T23:59:59.999Z']) width(`utc-${time.slice(0, 4)}`, 'suitable', data => {
    data.context.evaluatedAt = time; data.bundle.evidence[0].observedAt = time; data.bundle.evidence[0].review.reviewedAt = time;
  });
  add('slope/downhill-excluded', 'slopePercent', 'unsuitable', data => { data.bundle.observations[0].value = -8; });
  add('slope/downhill-boundary', 'slopePercent', 'suitable', data => { Object.assign(data.bundle.observations[0], { value: -6, uncertainty: 0 }); });
  add('slope/downhill-straddle', 'slopePercent', 'unknown', data => { data.bundle.observations[0].value = -6; });
  add('slope/uphill-unconstrained', 'slopePercent', 'suitable', data => { data.profile.hard.maxUphillPercent = null; data.bundle.observations[0].value = 80; });
  add('slope/downhill-unconstrained', 'slopePercent', 'suitable', data => { data.profile.hard.maxDownhillPercent = null; data.bundle.observations[0].value = -80; });
  add('steps/uncertain-count', 'stepCount', 'unknown', data => { data.bundle.observations[0].uncertainty = 1; });
  add('surface/case-sensitive', 'surface', 'unsuitable', data => { data.bundle.observations[0].value = 'Paved'; });
  add('surface/agreement', 'surface', 'suitable', data => { addObservation(data.bundle, 'surface', 'paved', null); });
  return cases.map(({ id, attribute, data, status }) => {
    const assessment = assessPath(data.bundle, data.profile, data.context, 'a', 'b', ['ab']);
    assert.equal(assessment.status, status, id);
    assert(assessment.reasons.every(reason => reason.attribute === attribute), id);
    return {
      id, synthetic: true, attribute, hard: data.profile.hard,
      observations: data.bundle.observations.map(({ edgeId: _, ...observation }) => observation),
      evidence: data.bundle.evidence.map(item => ({ id: item.id, source: item.source, actorId: item.actorId, method: item.method, observedAtMs: Date.parse(item.observedAt), validUntilMs: item.validUntil === null ? null : Date.parse(item.validUntil), review: { status: item.review.status, reviewerId: item.review.reviewerId, reviewedAtMs: item.review.reviewedAt === null ? null : Date.parse(item.review.reviewedAt) } })),
      policy: { acceptedSources: data.profile.evidencePolicy.acceptedSources, requireReviewed: data.profile.evidencePolicy.requireReviewed, maxAgeSeconds: data.profile.evidencePolicy.maxAgeSeconds },
      evaluatedAtMs: Date.parse(data.context.evaluatedAt),
      expected: { status: assessment.status, reasons: assessment.reasons.map(({ code, evidenceIds }) => ({ code, evidenceIds })), evidenceIds: assessment.evidenceIds },
    };
  });
}
