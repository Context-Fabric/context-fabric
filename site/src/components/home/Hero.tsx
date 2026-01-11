import Image from "next/image";
import Link from "next/link";

export function Hero() {
  return (
    <section className="pt-44 pb-20 px-6 md:px-10 max-w-[1200px] mx-auto">
      <div className="grid grid-cols-1 lg:grid-cols-[1fr_1.4fr] gap-12 items-center">
        {/* Content */}
        <div className="max-w-[480px] lg:max-w-none text-center lg:text-left mx-auto lg:mx-0">
          <div className="inline-block text-[0.8125rem] font-medium text-[var(--color-accent)] tracking-wide uppercase mb-6">
            AI-Native Corpus Infrastructure
          </div>
          <h1 className="text-[2.25rem] md:text-[3.25rem] leading-[1.15] mb-6 tracking-tight">
            From research notebooks to production APIs
          </h1>
          <p className="text-[1.125rem] text-[var(--color-text-secondary)] mb-8 leading-relaxed">
            Memory-efficient storage and querying for annotated text corpora.
            Compatible with 35+ Text-Fabric datasets. Scales from laptops to AI-powered pipelines.
          </p>
          <div className="flex gap-4 justify-center lg:justify-start">
            <Link
              href="/docs"
              className="btn-primary px-7 py-3.5 rounded-md text-[0.9375rem] font-medium transition-opacity"
            >
              Get Started
            </Link>
            <Link
              href="/docs"
              className="px-7 py-3.5 rounded-md text-[0.9375rem] font-medium bg-transparent text-[var(--color-text)] border border-[var(--color-border)] hover:border-[var(--color-text)] transition-colors"
            >
              Documentation
            </Link>
          </div>
        </div>

        {/* Terminal Demo */}
        <div>
          <div className="rounded-[10px] overflow-hidden border border-[var(--color-border)]">
            <Image
              src="/images/demo-terminal-large.gif"
              alt="Context-Fabric MCP Server Demo"
              width={800}
              height={600}
              className="w-full block"
              unoptimized
            />
          </div>
          <p className="text-center mt-4 text-[0.875rem] text-[var(--color-text-secondary)]">
            AI agent querying BHSA corpus via MCP protocol
          </p>
        </div>
      </div>
    </section>
  );
}
