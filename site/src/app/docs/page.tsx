import Link from "next/link";
import { getPackages } from "@/lib/docs";

export default function DocsPage() {
  const packages = getPackages();

  return (
    <article className="max-w-4xl">
      <header className="mb-12">
        <h1 className="font-serif text-[2.5rem] font-semibold text-[var(--color-text)] mb-4">
          Getting Started
        </h1>
        <p className="text-[1.125rem] text-[var(--color-text-secondary)] leading-relaxed">
          Context-Fabric is a graph-based corpus engine for annotated text. It provides
          tools for loading, navigating, and querying annotated text corpora using a
          graph-based data model.
        </p>
      </header>

      <section className="mb-12">
        <h2 className="text-xl font-semibold text-[var(--color-text)] mb-4 pb-2 border-b border-[var(--color-border)]">
          Installation
        </h2>
        <div className="bg-[#1F1F1F] text-white rounded-lg p-4 font-mono text-sm">
          <span className="text-[var(--color-accent)]">$</span> pip install context-fabric
        </div>
        <p className="mt-4 text-[var(--color-text-secondary)]">
          For MCP server support:
        </p>
        <div className="bg-[#1F1F1F] text-white rounded-lg p-4 font-mono text-sm mt-2">
          <span className="text-[var(--color-accent)]">$</span> pip install context-fabric[mcp]
        </div>
      </section>

      <section className="mb-12">
        <h2 className="text-xl font-semibold text-[var(--color-text)] mb-4 pb-2 border-b border-[var(--color-border)]">
          Quick Example
        </h2>
        <div className="bg-[#1F1F1F] text-white rounded-lg overflow-hidden">
          <div className="px-4 py-2 text-xs font-medium text-white/50 border-b border-white/10">
            Python
          </div>
          <pre className="p-4 text-sm overflow-x-auto">
            <code>{`import cfabric

# Load a corpus
CF = cfabric.Fabric('path/to/corpus')
api = CF.loadAll()

# Search for patterns
query = '''
  verse
    word lex=MLK
'''

for result in api.S.search(query):
    print(api.T.text(result))`}</code>
          </pre>
        </div>
      </section>

      <section className="mb-12">
        <h2 className="text-xl font-semibold text-[var(--color-text)] mb-4 pb-2 border-b border-[var(--color-border)]">
          API Reference
        </h2>
        <p className="text-[var(--color-text-secondary)] mb-6">
          Explore the full API documentation for each package:
        </p>
        <div className="grid gap-4">
          {packages.map((pkg) => (
            <Link
              key={pkg.name}
              href={pkg.path}
              className="group block p-4 rounded-lg border border-[var(--color-border)] hover:border-[var(--color-accent)] transition-colors bg-[var(--color-bg-alt)]"
            >
              <h3 className="font-mono text-lg font-semibold text-[var(--color-accent)] group-hover:underline mb-1">
                {pkg.name}
              </h3>
              <p className="text-sm text-[var(--color-text-secondary)]">
                {pkg.summary}
              </p>
            </Link>
          ))}
        </div>
      </section>

      <section>
        <h2 className="text-xl font-semibold text-[var(--color-text)] mb-4 pb-2 border-b border-[var(--color-border)]">
          Resources
        </h2>
        <ul className="space-y-3">
          <li>
            <Link
              href="/paper"
              className="text-[var(--color-accent)] hover:underline"
            >
              Introduction to Context-Fabric (Paper)
            </Link>
          </li>
          <li>
            <a
              href="https://github.com/Context-Fabric/context-fabric"
              target="_blank"
              rel="noopener noreferrer"
              className="text-[var(--color-accent)] hover:underline"
            >
              GitHub Repository →
            </a>
          </li>
          <li>
            <a
              href="https://pypi.org/project/context-fabric/"
              target="_blank"
              rel="noopener noreferrer"
              className="text-[var(--color-accent)] hover:underline"
            >
              PyPI Package →
            </a>
          </li>
        </ul>
      </section>
    </article>
  );
}
