export function CodeExamples() {
  return (
    <section
      id="code"
      className="py-24 px-6 md:px-10 bg-[#1F1F1F] text-white [&[data-theme='dark']]:bg-[#111111] [&[data-theme='dark']]:border-y [&[data-theme='dark']]:border-[var(--color-border)]"
    >
      <div className="max-w-[1000px] mx-auto">
        <div className="mb-16">
          <h2 className="text-[2.25rem] mb-3 tracking-tight text-white">
            Clean, familiar API
          </h2>
          <p className="text-[1.0625rem] text-white/60">
            If you know Text-Fabric, you know Context-Fabric. The same familiar API
            with dramatically better performance.
          </p>
        </div>

        <div className="grid grid-cols-1 md:grid-cols-2 gap-8">
          {/* Load and search */}
          <div className="bg-white/5 border border-white/10 rounded-lg overflow-hidden">
            <div className="px-5 py-3 text-[0.75rem] font-medium text-white/50 border-b border-white/10">
              Load and search
            </div>
            <pre className="p-6 text-[1rem] leading-[1.7] overflow-x-auto text-white/85">
              <code>
                <span className="text-[var(--color-accent)]">from</span> cfabric{" "}
                <span className="text-[var(--color-accent)]">import</span> Fabric{"\n"}
                {"\n"}
                <span className="text-white/35"># Load corpus with memory-mapped storage</span>
                {"\n"}
                CF = Fabric(<span className="text-[#9ECE6A]">&apos;path/to/bhsa&apos;</span>){"\n"}
                api = CF.<span className="text-[#7AA2F7]">loadAll</span>(){"\n"}
                {"\n"}
                <span className="text-white/35"># Search for patterns</span>
                {"\n"}
                query = <span className="text-[#9ECE6A]">&apos;&apos;&apos;</span>{"\n"}
                <span className="text-[#9ECE6A]">  verse</span>{"\n"}
                <span className="text-[#9ECE6A]">    word lex=MLK</span>{"\n"}
                <span className="text-[#9ECE6A]">&apos;&apos;&apos;</span>{"\n"}
                {"\n"}
                <span className="text-[var(--color-accent)]">for</span> result{" "}
                <span className="text-[var(--color-accent)]">in</span> api.S.
                <span className="text-[#7AA2F7]">search</span>(query):{"\n"}
                {"    "}
                <span className="text-[#7AA2F7]">print</span>(api.T.
                <span className="text-[#7AA2F7]">text</span>(result))
              </code>
            </pre>
          </div>

          {/* Navigate the graph */}
          <div className="bg-white/5 border border-white/10 rounded-lg overflow-hidden">
            <div className="px-5 py-3 text-[0.75rem] font-medium text-white/50 border-b border-white/10">
              Navigate the graph
            </div>
            <pre className="p-6 text-[1rem] leading-[1.7] overflow-x-auto text-white/85">
              <code>
                <span className="text-white/35"># Traverse linguistic hierarchy</span>
                {"\n"}
                <span className="text-[var(--color-accent)]">for</span> word{" "}
                <span className="text-[var(--color-accent)]">in</span> api.N.
                <span className="text-[#7AA2F7]">walk</span>(
                <span className="text-[#9ECE6A]">&apos;word&apos;</span>):{"\n"}
                {"    "}
                <span className="text-white/35"># Access features</span>
                {"\n"}
                {"    "}pos = api.F.sp.<span className="text-[#7AA2F7]">v</span>(word){"\n"}
                {"    "}lex = api.F.lex.<span className="text-[#7AA2F7]">v</span>(word){"\n"}
                {"\n"}
                {"    "}
                <span className="text-white/35"># Move up the hierarchy</span>
                {"\n"}
                {"    "}clause = api.L.<span className="text-[#7AA2F7]">u</span>(word,{" "}
                <span className="text-[#9ECE6A]">&apos;clause&apos;</span>){"\n"}
                {"\n"}
                {"    "}
                <span className="text-white/35"># Render text</span>
                {"\n"}
                {"    "}text = api.T.<span className="text-[#7AA2F7]">text</span>(word)
              </code>
            </pre>
          </div>
        </div>
      </div>
    </section>
  );
}
