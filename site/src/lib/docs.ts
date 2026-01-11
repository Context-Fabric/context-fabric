/**
 * Documentation utilities for Context-Fabric API docs
 */
import fs from "fs";
import path from "path";
import type {
  DocModule,
  DocClass,
  DocFunction,
  DocsIndex,
  NavItem,
  SearchItem,
} from "@/types/docs";

// Read documentation JSON files from the monorepo docs directory at build time
const docsPath = path.join(process.cwd(), "..", "docs", "api");

function loadJson<T>(filename: string): T {
  const filePath = path.join(docsPath, filename);
  const content = fs.readFileSync(filePath, "utf-8");
  return JSON.parse(content) as T;
}

const cfabricDocs = loadJson<DocModule>("cfabric.json");
const mcpDocs = loadJson<DocModule>("cfabric_mcp.json");
const benchmarksDocs = loadJson<DocModule>("cfabric_benchmarks.json");
const index = loadJson<DocsIndex>("index.json");

export const packages: Record<string, DocModule> = {
  cfabric: cfabricDocs as unknown as DocModule,
  cfabric_mcp: mcpDocs as unknown as DocModule,
  cfabric_benchmarks: benchmarksDocs as unknown as DocModule,
};

export const navigation: NavItem[] = index.navigation;

/**
 * Get a module by package name and path segments
 */
export function getModule(
  packageName: string,
  path: string[]
): DocModule | null {
  const pkg = packages[packageName];
  if (!pkg) return null;

  let current: DocModule = pkg;
  for (const segment of path) {
    const next = current.modules?.[segment];
    if (!next) return null;
    current = next;
  }
  return current;
}

/**
 * Get a class by package name, module path, and class name
 */
export function getClass(
  packageName: string,
  path: string[],
  className: string
): DocClass | null {
  const module = getModule(packageName, path);
  return module?.classes?.[className] ?? null;
}

/**
 * Get a function by package name, module path, and function name
 */
export function getFunction(
  packageName: string,
  path: string[],
  functionName: string
): DocFunction | null {
  const module = getModule(packageName, path);
  return module?.functions?.[functionName] ?? null;
}

/**
 * Get all packages with their top-level info
 */
export function getPackages(): Array<{
  name: string;
  summary: string;
  path: string;
}> {
  return Object.entries(packages).map(([name, pkg]) => ({
    name,
    summary: pkg.docstring?.summary || "",
    path: `/docs/api/${name}`,
  }));
}

/**
 * Build a flat list of all documentable items for search indexing
 */
export function buildSearchIndex(): SearchItem[] {
  const items: SearchItem[] = [];

  for (const [pkgName, pkg] of Object.entries(packages)) {
    function traverse(mod: DocModule, basePath: string) {
      // Add the module itself
      items.push({
        type: "module",
        name: mod.name,
        path: basePath,
        package: pkgName,
        summary: mod.docstring?.summary || "",
      });

      // Add classes
      for (const [name, cls] of Object.entries(mod.classes || {})) {
        items.push({
          type: "class",
          name,
          path: `${basePath}#${name}`,
          package: pkgName,
          summary: cls.docstring?.summary || "",
        });

        // Add methods
        for (const [methodName, method] of Object.entries(cls.methods || {})) {
          if (methodName.startsWith("_") && methodName !== "__init__") continue;
          items.push({
            type: "method",
            name: `${name}.${methodName}`,
            path: `${basePath}#${name}.${methodName}`,
            package: pkgName,
            summary: method.docstring?.summary || "",
          });
        }
      }

      // Add functions
      for (const [name, fn] of Object.entries(mod.functions || {})) {
        if (name.startsWith("_")) continue;
        items.push({
          type: "function",
          name,
          path: `${basePath}#${name}`,
          package: pkgName,
          summary: fn.docstring?.summary || "",
        });
      }

      // Recurse into submodules
      for (const [name, child] of Object.entries(mod.modules || {})) {
        traverse(child, `${basePath}/${name}`);
      }
    }

    traverse(pkg, `/docs/api/${pkgName}`);
  }

  return items;
}

/**
 * Generate static params for all documentation pages
 */
export function generateAllDocParams(): Array<{
  package: string;
  path: string[];
}> {
  const params: Array<{ package: string; path: string[] }> = [];

  for (const [pkgName, pkg] of Object.entries(packages)) {
    function traverse(mod: DocModule, path: string[]) {
      params.push({ package: pkgName, path });
      for (const [name, child] of Object.entries(mod.modules || {})) {
        traverse(child, [...path, name]);
      }
    }
    traverse(pkg, []);
  }

  return params;
}
