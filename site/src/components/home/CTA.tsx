export function CTA() {
  return (
    <section className="py-24 px-6 md:px-10 text-center bg-[var(--color-bg-alt)] border-t border-[var(--color-border)]">
      <h2 className="text-[2.25rem] mb-4 tracking-tight">Ready to explore?</h2>
      <p className="text-[1.0625rem] text-[var(--color-text-secondary)] mb-8 max-w-[480px] mx-auto">
        Install Context-Fabric and start querying linguistic corpora in minutes.
      </p>
      <div className="inline-flex items-center gap-3 bg-[var(--color-text)] text-white px-6 py-4 rounded-lg font-mono text-[0.9375rem] [data-theme='dark']:bg-[var(--color-bg-alt)] [data-theme='dark']:border [data-theme='dark']:border-[var(--color-border)]">
        <span className="text-[var(--color-accent)]">$</span> pip install context-fabric
      </div>
    </section>
  );
}
