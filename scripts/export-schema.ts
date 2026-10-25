import assert from 'node:assert/strict';
import { mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { bundleSchema, profileSchema, contextSchema } from '../packages/mobility-contract/src/schema';

const directory = new URL('../packages/mobility-contract/schema/', import.meta.url);
if (!process.argv.includes('--check')) mkdirSync(directory, { recursive: true });
for (const [name, schema] of Object.entries({ bundle: bundleSchema, profile: profileSchema, context: contextSchema })) {
  const url = new URL(`${name}-v0.1.0-draft.1.schema.json`, directory);
  const text = JSON.stringify(schema, null, 2) + '\n';
  if (process.argv.includes('--check')) assert.equal(readFileSync(url, 'utf8'), text, `${name}: exported schema is stale`);
  else writeFileSync(url, text);
}
console.log('Three versioned contract schemas are current.');
