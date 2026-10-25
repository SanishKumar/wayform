import { createHash } from 'node:crypto';
import {
  prepareBuildingCompilation,
  stableJson,
  type CompiledBuildingPackage,
} from './compilerCore';
import type { ValidationReport } from './validation';

export * from './compilerCore';

export interface CompilationResult {
  package: CompiledBuildingPackage | null;
  report: ValidationReport;
}

export function compileBuilding(value: unknown): CompilationResult {
  const preparation = prepareBuildingCompilation(value);
  if (!preparation.content) {
    return { package: null, report: preparation.report };
  }

  const contentHash = createHash('sha256').update(stableJson(preparation.content)).digest('hex');
  return {
    package: {
      ...preparation.content,
      manifest: { hashAlgorithm: 'sha256', contentHash },
    },
    report: preparation.report,
  };
}
