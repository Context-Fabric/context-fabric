# Integrating API Documentation with Next.js

This directory contains auto-generated JSON API documentation for Context-Fabric packages. This guide explains how to consume these files in a Next.js website.

## Generated Files

| File | Description |
|------|-------------|
| `index.json` | Combined metadata, navigation structure, and links to all packages |
| `cfabric.json` | Core library API documentation |
| `cfabric_mcp.json` | MCP server API documentation |
| `cfabric_benchmarks.json` | Benchmarks suite API documentation |

## JSON Structure

Each package JSON follows this structure:

```json
{
  "name": "cfabric",
  "kind": "module",
  "path": "cfabric",
  "docstring": {
    "summary": "Short description",
    "description": "Full docstring text",
    "parsed": []
  },
  "modules": {
    "core": {
      "name": "core",
      "classes": {
        "Fabric": {
          "name": "Fabric",
          "docstring": {...},
          "methods": {...},
          "attributes": {...}
        }
      },
      "functions": {...},
      "modules": {...}
    }
  },
  "aliases": {...}
}
```

## Integration Options

### Option A: Static Import (Recommended for SSG)

If documentation doesn't change frequently, import JSON directly:

```typescript
// lib/docs.ts
import cfabricDocs from '@/docs/api/cfabric.json';
import mcpDocs from '@/docs/api/cfabric_mcp.json';
import benchmarksDocs from '@/docs/api/cfabric_benchmarks.json';
import indexData from '@/docs/api/index.json';

export const packages = {
  cfabric: cfabricDocs,
  cfabric_mcp: mcpDocs,
  cfabric_benchmarks: benchmarksDocs,
};

export const navigation = indexData.navigation;

export function getModule(packageName: string, path: string[]) {
  let current: any = packages[packageName];
  for (const segment of path) {
    current = current?.modules?.[segment];
    if (!current) return null;
  }
  return current;
}

export function getClass(packageName: string, path: string[], className: string) {
  const module = getModule(packageName, path);
  return module?.classes?.[className] ?? null;
}
```

### Option B: Dynamic Loading (For Large Docs)

Load documentation on demand via API routes:

```typescript
// app/api/docs/[...path]/route.ts
import { NextRequest, NextResponse } from 'next/server';
import fs from 'fs/promises';
import path from 'path';

export async function GET(
  request: NextRequest,
  { params }: { params: { path: string[] } }
) {
  const [packageName, ...rest] = params.path;

  try {
    const filePath = path.join(process.cwd(), 'docs/api', `${packageName}.json`);
    const content = await fs.readFile(filePath, 'utf-8');
    const data = JSON.parse(content);

    // Navigate to nested path if specified
    let result = data;
    for (const segment of rest) {
      result = result?.modules?.[segment];
      if (!result) {
        return NextResponse.json({ error: 'Not found' }, { status: 404 });
      }
    }

    return NextResponse.json(result);
  } catch {
    return NextResponse.json({ error: 'Not found' }, { status: 404 });
  }
}
```

### Option C: Git Submodule (Separate Website Repo)

If your website is in a separate repository:

```bash
# Add context-fabric as a submodule
git submodule add https://github.com/Context-Fabric/context-fabric.git external/context-fabric

# Reference docs in next.config.js or import from submodule path
```

Or fetch from GitHub raw URLs at build time:

```typescript
// lib/docs.ts
const GITHUB_RAW = 'https://raw.githubusercontent.com/Context-Fabric/context-fabric/master/docs/api';

export async function fetchPackageDocs(packageName: string) {
  const res = await fetch(`${GITHUB_RAW}/${packageName}.json`);
  if (!res.ok) throw new Error(`Failed to fetch ${packageName} docs`);
  return res.json();
}
```

## Example Page Components

### Sidebar Navigation

```tsx
// components/docs/Sidebar.tsx
import Link from 'next/link';
import { navigation } from '@/lib/docs';

export function Sidebar({ currentPath }: { currentPath: string }) {
  return (
    <nav className="w-64 p-4">
      {navigation.map((pkg) => (
        <div key={pkg.title} className="mb-4">
          <Link
            href={pkg.path}
            className="font-semibold text-lg"
          >
            {pkg.title}
          </Link>
          <ul className="ml-4 mt-2">
            {pkg.children?.map((child) => (
              <li key={child.path}>
                <Link
                  href={child.path}
                  className={currentPath === child.path ? 'text-blue-600' : ''}
                >
                  {child.title}
                </Link>
              </li>
            ))}
          </ul>
        </div>
      ))}
    </nav>
  );
}
```

### Class Documentation

```tsx
// components/docs/ClassDoc.tsx
interface ClassData {
  name: string;
  docstring: { summary: string; description: string };
  methods: Record<string, MethodData>;
  attributes: Record<string, AttributeData>;
}

export function ClassDoc({ cls }: { cls: ClassData }) {
  return (
    <article className="mb-8">
      <h2 className="text-2xl font-bold font-mono">{cls.name}</h2>

      {cls.docstring?.summary && (
        <p className="mt-2 text-gray-700">{cls.docstring.summary}</p>
      )}

      {Object.keys(cls.methods).length > 0 && (
        <section className="mt-6">
          <h3 className="text-xl font-semibold">Methods</h3>
          {Object.values(cls.methods).map((method) => (
            <MethodDoc key={method.name} method={method} />
          ))}
        </section>
      )}
    </article>
  );
}
```

### Method Documentation

```tsx
// components/docs/MethodDoc.tsx
interface MethodData {
  name: string;
  signature: string;
  docstring: { summary: string; description: string };
  parameters: Array<{ name: string; type: string; default?: string }>;
  returns: { type: string };
}

export function MethodDoc({ method }: { method: MethodData }) {
  return (
    <div className="mt-4 p-4 bg-gray-50 rounded">
      <code className="text-sm">
        <span className="text-blue-600">{method.name}</span>
        <span className="text-gray-600">{method.signature}</span>
        {method.returns?.type && (
          <span className="text-gray-500"> → {method.returns.type}</span>
        )}
      </code>

      {method.docstring?.summary && (
        <p className="mt-2">{method.docstring.summary}</p>
      )}

      {method.parameters.length > 0 && (
        <div className="mt-3">
          <h4 className="font-medium text-sm">Parameters</h4>
          <ul className="mt-1 text-sm">
            {method.parameters
              .filter(p => p.name !== 'self')
              .map((param) => (
                <li key={param.name} className="ml-4">
                  <code>{param.name}</code>
                  {param.type && <span className="text-gray-500">: {param.type}</span>}
                  {param.default && <span className="text-gray-400"> = {param.default}</span>}
                </li>
              ))}
          </ul>
        </div>
      )}
    </div>
  );
}
```

## Dynamic Routes

Set up catch-all routes for documentation pages:

```
app/
└── docs/
    └── api/
        └── [package]/
            └── [...path]/
                └── page.tsx
```

```tsx
// app/docs/api/[package]/[...path]/page.tsx
import { getModule, packages } from '@/lib/docs';
import { ClassDoc } from '@/components/docs/ClassDoc';
import { notFound } from 'next/navigation';

interface Props {
  params: { package: string; path: string[] };
}

export async function generateStaticParams() {
  const params: { package: string; path: string[] }[] = [];

  for (const [pkgName, pkg] of Object.entries(packages)) {
    function traverse(mod: any, path: string[]) {
      params.push({ package: pkgName, path });
      for (const [name, child] of Object.entries(mod.modules || {})) {
        traverse(child, [...path, name]);
      }
    }
    traverse(pkg, []);
  }

  return params;
}

export default function ApiDocPage({ params }: Props) {
  const module = getModule(params.package, params.path);

  if (!module) {
    notFound();
  }

  return (
    <main className="p-8">
      <h1 className="text-3xl font-bold">{module.name}</h1>

      {module.docstring?.description && (
        <p className="mt-4 text-gray-700">{module.docstring.description}</p>
      )}

      {Object.values(module.classes || {}).map((cls: any) => (
        <ClassDoc key={cls.name} cls={cls} />
      ))}
    </main>
  );
}
```

## Search Integration

Add full-text search with Fuse.js:

```typescript
// lib/search.ts
import Fuse from 'fuse.js';
import { packages } from './docs';

interface SearchItem {
  type: 'module' | 'class' | 'function' | 'method';
  name: string;
  path: string;
  package: string;
  summary: string;
}

function buildSearchIndex(): SearchItem[] {
  const items: SearchItem[] = [];

  for (const [pkgName, pkg] of Object.entries(packages)) {
    function traverse(mod: any, path: string) {
      items.push({
        type: 'module',
        name: mod.name,
        path,
        package: pkgName,
        summary: mod.docstring?.summary || '',
      });

      for (const [name, cls] of Object.entries(mod.classes || {}) as any) {
        items.push({
          type: 'class',
          name,
          path: `${path}#${name}`,
          package: pkgName,
          summary: cls.docstring?.summary || '',
        });
      }

      for (const [name, child] of Object.entries(mod.modules || {})) {
        traverse(child, `${path}/${name}`);
      }
    }

    traverse(pkg, `/docs/api/${pkgName}`);
  }

  return items;
}

const fuse = new Fuse(buildSearchIndex(), {
  keys: ['name', 'summary'],
  threshold: 0.3,
});

export function searchDocs(query: string) {
  return fuse.search(query).map(r => r.item);
}
```

## Regenerating Documentation

Documentation is auto-generated by GitHub Actions when Python files change. To regenerate manually:

```bash
# Install dependencies
pip install griffe griffe-typingdoc
pip install -e libs/core -e libs/mcp -e libs/benchmarks

# Generate documentation
python scripts/docs/generate_docs.py
```

## TypeScript Types

For type safety, you can generate types from the JSON structure:

```typescript
// types/docs.ts
export interface DocModule {
  name: string;
  kind: 'module';
  path: string;
  docstring: Docstring;
  classes: Record<string, DocClass>;
  functions: Record<string, DocFunction>;
  modules: Record<string, DocModule>;
  aliases: Record<string, { name: string; target: string }>;
}

export interface Docstring {
  summary: string;
  description: string;
  parsed: any[];
}

export interface DocClass {
  name: string;
  kind: 'class';
  path: string;
  docstring: Docstring;
  bases: string[];
  methods: Record<string, DocFunction>;
  attributes: Record<string, DocAttribute>;
}

export interface DocFunction {
  name: string;
  kind: 'function';
  signature: string;
  docstring: Docstring;
  parameters: DocParameter[];
  returns: { type: string };
  decorators: any[];
}

export interface DocParameter {
  name: string;
  type: string;
  default: string | null;
  kind: string;
}

export interface DocAttribute {
  name: string;
  type: string;
  docstring: Docstring;
  value: any;
}
```
