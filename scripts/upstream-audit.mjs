import { execFileSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';
const cwd = fileURLToPath(new URL('../compat/voicegis/upstream/', import.meta.url));
for (const venue of ['reference-medical-centre', 'asterion-medical-center', 'harbor-exchange', 'meridian-park-campus']) {
  execFileSync(process.execPath, ['--import', 'tsx', 'scripts/auditDirections.ts', venue], { cwd, stdio: 'inherit' });
}
