import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { assessPath } from '../packages/mobility-contract/src/assess';
import { inputDigests } from '../packages/mobility-contract/src/identity';
import { applyEdits, type Edit } from './loader-cases';
import type { EvaluationContext, MobilityBundle, MobilityProfile, PathAssessment } from '../packages/mobility-contract/src/types';

type Data = { bundle: MobilityBundle; profile: MobilityProfile; context: EvaluationContext };
type Request = Data & { protocolVersion: string; requestId: string; operation: 'assess-path'; startId: string; destinationId: string; edgeIds: string[] };
type Case = Data & { id: string; request: Pick<Request, 'startId' | 'destinationId' | 'edgeIds'>; expected: PathAssessment };
type Response = { protocolVersion: string; requestId: string; snapshot: ReturnType<typeof inputDigests> | null; outcome: string; assessment?: PathAssessment; error?: { code: string; document: string | null } };
const examples = new URL('../packages/mobility-contract/examples/', import.meta.url);
const portable = JSON.parse(readFileSync(new URL('decisions-v0.1.0-draft.1.json', examples), 'utf8')) as { examples: Case[] };
const paths = readFileSync(new URL('paths-v0.1.0-draft.1.jsonl', examples), 'utf8').trim().split('\n').map(line => JSON.parse(line) as Case);
const loading = JSON.parse(readFileSync(new URL('loading-v0.1.0-draft.1.json', examples), 'utf8')) as { base: Data; cases: { id: string; edits: Edit[]; valid: boolean; expected: { digests: ReturnType<typeof inputDigests>; assessment: PathAssessment } | null }[] };

function permute(data: Data): Data {
  const result = structuredClone(data);
  for (const key of ['frames', 'nodes', 'edges', 'evidence', 'observations', 'transforms', 'policyRequirements'] as const) result.bundle[key].reverse();
  for (const transform of result.bundle.transforms) transform.controls.reverse();
  for (const requirement of result.bundle.policyRequirements) requirement.edgeIds.reverse();
  result.profile.preferences.avoidKinds.reverse(); result.profile.evidencePolicy.acceptedSources.reverse();
  result.context.policies.reverse(); result.context.trustedIssuerIds.reverse();
  return result;
}

const requests: Request[] = [];
const expected: { digests: ReturnType<typeof inputDigests> | null; assessment: PathAssessment | null; document?: string }[] = [];
for (const item of [...portable.examples, ...paths]) {
  for (const reversed of [false, true]) {
    const data = reversed ? permute(item) : item;
    const assessment = assessPath(data.bundle, data.profile, data.context, item.request.startId, item.request.destinationId, item.request.edgeIds);
    assert.deepEqual(assessment, item.expected, item.id);
    requests.push({ protocolVersion: '0.1.0', requestId: `${item.id}${reversed ? '-permuted' : ''}`, operation: 'assess-path', bundle: data.bundle, profile: data.profile, context: data.context, ...item.request });
    expected.push({ digests: inputDigests(data.bundle, data.profile, data.context), assessment });
  }
}
for (const item of loading.cases) {
  const data = applyEdits(loading.base, item.edits);
  requests.push({ ...data, protocolVersion: '0.1.0', requestId: `loading:${item.id}`, operation: 'assess-path', startId: 'a', destinationId: 'b', edgeIds: ['ab'] });
  expected.push({ digests: item.expected?.digests ?? null, assessment: item.expected?.assessment ?? null, document: item.valid ? undefined : String(item.edits[0].path[0]) });
}

const executable = process.env.WAYFORM_MOBILITY_BIN ?? resolve(process.env.CARGO_TARGET_DIR ?? 'target', 'debug', process.platform === 'win32' ? 'wayform-mobility.exe' : 'wayform-mobility');
const run = spawnSync(executable, [], { input: requests.map(item => JSON.stringify(item)).join('\n') + '\n', encoding: 'utf8', timeout: 30_000, maxBuffer: 16 * 1024 * 1024 });
assert.ifError(run.error); assert.equal(run.status, 0, run.stderr); assert.equal(run.stderr, '');
const responses = run.stdout.trim().split('\n').map(line => JSON.parse(line) as Response);
assert.equal(responses.length, requests.length);
for (const [index, response] of responses.entries()) {
  const item = requests[index], expectation = expected[index];
  assert.equal(response.protocolVersion, item.protocolVersion, item.requestId);
  assert.equal(response.requestId, item.requestId);
  assert.deepEqual(response.snapshot, expectation.digests, item.requestId);
  if (expectation.assessment) {
    assert.equal(response.outcome, 'assessed', item.requestId);
    assert.deepEqual(response.assessment, expectation.assessment, item.requestId);
  } else {
    assert.equal(response.outcome, 'rejected', item.requestId);
    assert.equal(response.error?.code, 'invalid-contract', item.requestId);
    assert.equal(response.error?.document, expectation.document, item.requestId);
    assert.equal(response.assessment, undefined, item.requestId);
  }
}
console.log(`Native mobility conformance: ${requests.length} requests passed. Rust validates documents and produces complete supplied-path receipts.`);
