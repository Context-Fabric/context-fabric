const features = [
  {
    number: "01",
    title: "Memory-Mapped Storage",
    description:
      "Data lives on disk, not in memory, so you can load more corpora at once. In production, multiple workers share the same pages. 92% less memory whether you're running locally or at scale.",
  },
  {
    number: "02",
    title: "AI Agent Integration",
    description:
      "Built-in MCP server enables Claude, ChatGPT, Cursor, and any MCP-compatible agent to query corpora through natural language. Bringing computational linguistics to conversational interfaces.",
  },
  {
    number: "03",
    title: "Graph-Based Model",
    description:
      "Nodes represent textual units—morphemes, words, phrases, clauses. Edges capture syntax, coreference, discourse. Navigate with elegant traversal APIs.",
  },
  {
    number: "04",
    title: "Pattern Search",
    description:
      "SPIN algorithm finds complex linguistic patterns across massive corpora. Query by lemma, part-of-speech, syntactic role—instant results.",
  },
];

export function Features() {
  return (
    <section
      id="features"
      className="py-24 px-6 md:px-10 bg-[var(--color-bg-alt)] border-t border-[var(--color-border)]"
    >
      <div className="max-w-[1000px] mx-auto">
        <div className="mb-16">
          <h2 className="text-[2.25rem] mb-3 tracking-tight">
            Built for corpus linguistics at any scale
          </h2>
          <p className="text-[1.0625rem] text-[var(--color-text-secondary)]">
            Whether you&apos;re exploring in a notebook or deploying at scale, the same
            architecture delivers.
          </p>
        </div>

        <div className="flex flex-col">
          {features.map((feature) => (
            <div
              key={feature.number}
              className="grid grid-cols-1 md:grid-cols-[60px_1fr] gap-3 md:gap-8 py-10 border-t border-[var(--color-border)]"
            >
              <div className="font-serif text-[1.75rem] text-[var(--color-accent)]">
                {feature.number}
              </div>
              <div>
                <h3 className="text-[1.375rem] mb-2.5">{feature.title}</h3>
                <p className="text-[1rem] text-[var(--color-text-secondary)] leading-[1.75] max-w-[600px]">
                  {feature.description}
                </p>
              </div>
            </div>
          ))}
        </div>
      </div>
    </section>
  );
}
