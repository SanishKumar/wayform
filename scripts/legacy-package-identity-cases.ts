import { createHash } from 'node:crypto';
import { stablePackageJson } from '../compat/voicegis/upstream/src/data/packageLifecycle';

export function legacyPackageIdentityCases() {
  const inputs = [
    { z: [], a: {}, manifest: { contentHash: 'ignored' } },
    { aB: 1, ab: 2, Ab: 3, AB: 4, a10: 5, a2: 6, alpha: 7, Alpha: 8 },
    { label: '入口 · Café 😀\n\t"\\', values: [0, -0, 1e-7, 1e-6, 1e20, 1e21, Number.MIN_VALUE, Number.MAX_VALUE] },
    { nested: { manifest: { value: true }, array: [null, false, { z: 3.5, a: 'end' }] } },
  ];
  return inputs.map((value, index) => {
    const { manifest: _, ...content } = value as Record<string, unknown>;
    // Fixtures travel as JSON, which represents negative zero as zero.
    return { id: `identity-${index}`, value: JSON.parse(JSON.stringify(value)), hash: createHash('sha256').update(stablePackageJson(content)).digest('hex') };
  });
}
