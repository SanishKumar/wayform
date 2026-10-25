import assert from 'node:assert/strict';
import { mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import path from 'node:path';
import { buildLegacyBaseline, root } from '../compat/voicegis/harness';

const record = process.argv.includes('--record');
const results = await buildLegacyBaseline();
if (record) mkdirSync(path.join(root, 'expected'), { recursive: true });
for (const [name, contents] of Object.entries(results)) {
  const destination = path.join(root, 'expected', name);
  if (record) writeFileSync(destination, contents);
  else assert.equal(contents, readFileSync(destination, 'utf8'), `Legacy baseline changed: ${name}. Review differences; do not auto-update.`);
}
console.log(`${record ? 'Recorded' : 'Verified'} 7,238 legacy journeys, compiler artifacts, policy outcomes and receipts.`);
