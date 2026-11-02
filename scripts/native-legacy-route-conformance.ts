import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { PINNED_COMMIT, canonical, loadVenue, root, sha256, venues, verifySourceSnapshot } from '../compat/voicegis/harness';
import { createCompiledBuildingRuntime } from '../compat/voicegis/upstream/src/data/compiledBuilding';
import { calculateCompiledRoute, type ExplainedRouteResult } from '../compat/voicegis/upstream/src/engine/compiledRoutePolicy';
import { comparableSteps } from './legacy-direction-cases';

assert.equal(verifySourceSnapshot(), 35);
const baseline = readFileSync(resolve(root,'expected/route-digests.jsonl'),'utf8').trim().split('\n').map(line => JSON.parse(line) as { id:string; startId:string; destinationId:string; profile:'standard'|'wheelchair'; sha256:string });
const executable = process.env.WAYFORM_LEGACY_SEARCH_BIN ?? resolve(process.env.CARGO_TARGET_DIR ?? 'target','debug',process.platform === 'win32' ? 'wayform-legacy-search.exe' : 'wayform-legacy-search');
const comparable = (route:ExplainedRouteResult) => route.found ? { ...route, steps:comparableSteps(route.steps) } : route;
let total=0;
for (const venue of venues) {
  const pkg=await loadVenue(venue), runtime=createCompiledBuildingRuntime(pkg);
  const rows=baseline.filter(row => row.id.startsWith(`${venue}/`));
  const expected=rows.map(row => {
    const route=calculateCompiledRoute(runtime,row.startId,row.destinationId,{profile:row.profile});
    assert.equal(sha256(canonical(route)),row.sha256,row.id);
    return JSON.parse(JSON.stringify(comparable(route))) as ExplainedRouteResult;
  });
  const batches=[];
  for(let offset=0;offset<rows.length;offset+=100) {
    batches.push({protocolVersion:'0.4.0',sourceCommit:PINNED_COMMIT,requestId:`${venue}/${offset}`,venuePackage:pkg,
      queries:rows.slice(offset,offset+100).map(row => ({queryId:row.id,startId:row.startId,destinationId:row.destinationId,options:{accessibleOnly:row.profile==='wheelchair',allowRestricted:false},policy:{mode:'none'}}))});
  }
  const run=spawnSync(executable,[],{input:batches.map(b=>JSON.stringify(b)).join('\n')+'\n',encoding:'utf8',timeout:90_000,maxBuffer:128*1024*1024});
  assert.ifError(run.error); assert.equal(run.status,0,run.stderr); assert.equal(run.stderr,'');
  const responses=run.stdout.trim().split('\n').map(line=>JSON.parse(line) as {protocolVersion:string;sourceCommit:string;requestId:string;outcome:string;results:(ExplainedRouteResult & {queryId:string})[]});
  assert.equal(responses.length,batches.length);
  let index=0;
  for(const [batchIndex,response] of responses.entries()) {
    assert.equal(response.protocolVersion,'0.4.0'); assert.equal(response.sourceCommit,PINNED_COMMIT);
    assert.equal(response.requestId,batches[batchIndex].requestId); assert.equal(response.outcome,'completed',JSON.stringify(response));
    assert.equal(response.results.length,batches[batchIndex].queries.length);
    for(const result of response.results) {
      const {queryId,...route}=result;
      assert.equal(queryId,rows[index].id);
      assert.deepEqual(comparable(route),expected[index],queryId);
      index++; total++;
    }
  }
  assert.equal(index,rows.length);
}
assert.equal(total,7238);
console.log(`Native complete legacy results: all ${total} pinned journeys match paths, metadata, landmark directions, graphic assessments and receipts. Bearings use the documented nine-decimal comparison boundary.`);
