const stats = [
  { value: "92%*", label: "Less Memory" },
  { value: "12x*", label: "Faster Loading" },
  { value: "35+", label: "Corpora" },
  { value: "Zero", label: "Data Copies" },
];

export function Stats() {
  return (
    <section className="py-16 px-6 md:px-10 bg-[var(--color-bg)] border-t border-[var(--color-border)]">
      <div className="grid grid-cols-2 md:grid-cols-4 gap-8 max-w-[900px] mx-auto text-center">
        {stats.map((stat) => (
          <div key={stat.label}>
            <h3 className="font-serif text-[3rem] font-semibold text-[var(--color-accent)] mb-1.5">
              {stat.value}
            </h3>
            <p className="text-[0.875rem] font-medium text-[var(--color-text-secondary)] uppercase tracking-wider">
              {stat.label}
            </p>
          </div>
        ))}
      </div>
      <p className="text-center text-[0.8125rem] text-[var(--color-text-secondary)] mt-10 max-w-[900px] mx-auto">
        *Compared to Text-Fabric with BHSA
      </p>
    </section>
  );
}
