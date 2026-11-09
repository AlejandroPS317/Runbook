import type { Package, Script } from "./ipc/types";

/**
 * Part of the script list to show: everything, or the packages under `apps/` or
 * `packages/`, the folders Turborepo and most pnpm monorepos use.
 */
export type Section = "all" | "apps" | "packages";

export function inSection(pkg: Package, section: Section): boolean {
  switch (section) {
    case "all":
      return true;
    case "apps":
      return pkg.relativeDir.startsWith("apps/");
    case "packages":
      return pkg.relativeDir.startsWith("packages/");
    default: {
      const unreachable: never = section;
      return unreachable;
    }
  }
}

export function scriptCount(packages: Package[], section: Section): number {
  return packages.filter((pkg) => inSection(pkg, section)).reduce((sum, pkg) => sum + pkg.scripts.length, 0);
}

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
