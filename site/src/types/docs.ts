/**
 * TypeScript types for Context-Fabric API documentation
 */

export interface Docstring {
  summary: string;
  description: string;
  parsed?: unknown[];
  sections?: Record<string, unknown>;
}

export interface DocParameter {
  name: string;
  type: string;
  default: string | null;
  kind: string;
}

export interface DocFunction {
  name: string;
  kind: "function";
  signature: string;
  docstring: Docstring;
  parameters: DocParameter[];
  returns: { type: string };
  decorators: unknown[];
}

export interface DocAttribute {
  name: string;
  type: string;
  docstring: Docstring;
  value: unknown;
}

export interface DocClass {
  name: string;
  kind: "class";
  path: string;
  docstring: Docstring;
  bases: string[];
  methods: Record<string, DocFunction>;
  attributes: Record<string, DocAttribute>;
}

export interface DocModule {
  name: string;
  kind: "module";
  path: string;
  docstring: Docstring;
  classes: Record<string, DocClass>;
  functions: Record<string, DocFunction>;
  modules: Record<string, DocModule>;
  aliases: Record<string, { name: string; target: string }>;
}

export interface NavItem {
  title: string;
  path: string;
  children?: NavItem[];
}

export interface DocsIndex {
  generated_at: string;
  packages: Record<string, DocModule>;
  navigation: NavItem[];
}

export interface SearchItem {
  type: "module" | "class" | "function" | "method";
  name: string;
  path: string;
  package: string;
  summary: string;
}
