import type { Package, Script } from "./ipc/types";

/**
 * Packages with only the scripts matching `query`. Every whitespace-separated word must
 * appear (case-insensitively) in the package name or folder, the script name or its
 * command. Packages without matching scripts are dropped; an empty query keeps everything.
 */
export function filterPackages(packages: Package[], query: string): Package[] {
  const words = query.toLowerCase().split(/\s+/).filter(Boolean);
  if (words.length === 0) return packages;
  return packages
    .map((pkg) => {
      const owner = `${pkg.name ?? ""} ${pkg.relativeDir}`.toLowerCase();
      const matches = (script: Script) => {
        const text = `${owner} ${script.name} ${script.command}`.toLowerCase();
        return words.every((word) => text.includes(word));
      };
      return { ...pkg, scripts: pkg.scripts.filter(matches) };
    })
    .filter((pkg) => pkg.scripts.length > 0);
}
