// Explicit maintenance operation. Reads committed blobs; never the working tree.
import { execFileSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import { mkdirSync, writeFileSync, existsSync } from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const commit = '305c70bd418d7fd2b6f0ebf7a2ecd0ddcb9b9b7f';
const repository = 'https://github.com/SanishKumar/voicegis-indoor-ar';
const source = process.argv[2];
if (!source) throw new Error('Usage: node scripts/import-voicegis.mjs <local-VoiceGIS-repository>');
const root = fileURLToPath(new URL('../compat/voicegis/', import.meta.url));
if (existsSync(path.join(root, 'source-lock.json'))) throw new Error('Pinned import already exists; review an explicit replacement instead of overwriting it.');
const git = (...args) => execFileSync('git', ['-C', source, ...args], { maxBuffer: 32 * 1024 * 1024 });
const paths = new Set(git('ls-tree', '-r', '--name-only', commit).toString('utf8').trim().split('\n'));
const queue = [
  'README.md', 'scripts/auditDirections.ts', 'src/data/venuePackageContract.ts',
  'src/navigation/operationalLease.ts', 'packages/map-compiler/package.json',
  'packages/map-compiler/src/browser.ts', 'packages/spatial-schema/package.json',
  'buildings/asterion-medical-center/operations/all-public-lifts-closed.overlay.json',
];
for (const venue of ['reference-medical-centre', 'asterion-medical-center', 'harbor-exchange', 'meridian-park-campus']) {
  for (const suffix of ['source/building.json', 'compiled/building.package.json', 'compiled/validation-report.json']) {
    queue.push(`buildings/${venue}/${suffix}`);
  }
}
const files = [];
const seen = new Set();
while (queue.length) {
  const name = queue.shift();
  if (seen.has(name)) continue;
  if (!paths.has(name)) throw new Error(`Missing committed source: ${name}`);
  seen.add(name);
  const bytes = git('show', `${commit}:${name}`);
  const destination = path.join(root, 'upstream', name);
  mkdirSync(path.dirname(destination), { recursive: true });
  writeFileSync(destination, bytes);
  files.push({ path: name, bytes: bytes.length, sha256: createHash('sha256').update(bytes).digest('hex'), gitBlob: git('rev-parse', `${commit}:${name}`).toString().trim() });
  if (!name.endsWith('.ts')) continue;
  for (const match of bytes.toString().matchAll(/(?:from\s+|import\s*)['"]([^'"]+)['"]/g)) {
    const specifier = match[1];
    let target;
    if (specifier.startsWith('.')) target = path.posix.normalize(path.posix.join(path.posix.dirname(name), specifier));
    else if (specifier === '@voicegis/map-compiler') target = 'packages/map-compiler/src/index';
    else if (specifier === '@voicegis/spatial-schema') target = 'packages/spatial-schema/src/index';
    else if (specifier.startsWith('node:') || specifier.startsWith('ajv/')) continue;
    else throw new Error(`Unreviewed dependency ${specifier} in ${name}`);
    const resolved = [target, `${target}.ts`, `${target}/index.ts`].find(candidate => paths.has(candidate));
    if (!resolved) throw new Error(`Cannot resolve ${specifier} in ${name}`);
    queue.push(resolved);
  }
}
files.sort((a, b) => a.path < b.path ? -1 : a.path > b.path ? 1 : 0);
writeFileSync(path.join(root, 'source-lock.json'), JSON.stringify({ formatVersion: 1, repository, commit, extraction: 'Exact Git blobs; transitive source subset, not a full application migration.', files }, null, 2) + '\n');
console.log(`Pinned ${files.length} files from ${commit}.`);
