import { writeFileSync } from 'node:fs';
import { verifySourceSnapshot } from '../compat/voicegis/harness';
import { buildLegacyLeaseCases } from './legacy-lease-cases';
if (!process.argv.includes('--record')) throw new Error('Pass --record to replace legacy lease expectations.');
verifySourceSnapshot();
const corpus = await buildLegacyLeaseCases();
// Keep one trace per line, with each package projection shared by its group.
const groups = corpus.groups.map(group => `    {"id":${JSON.stringify(group.id)},"package":${JSON.stringify(group.package)},"cases":[\n${group.cases.map(item => '      ' + JSON.stringify(item)).join(',\n')}\n    ]}`);
writeFileSync(new URL('../compat/voicegis/expected/operational-leases.json', import.meta.url), `{"formatVersion":${corpus.formatVersion},"sourceCommit":${JSON.stringify(corpus.sourceCommit)},"groups":[\n${groups.join(',\n')}\n]}\n`);
