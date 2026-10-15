import Link from "next/link";
import { InstallCommands, type InstallGroup } from "./InstallCommands";

const RELEASE_NOTES_URL =
  "https://github.com/Context-Fabric/context-fabric/blob/cf-rust/libs/core/CHANGELOG.md";

const releaseStats = [
  { value: "9×", label: "Less memory" },
  { value: "11×", label: "Faster search" },
  { value: "0.003s", label: "To load a corpus" },
];

const installGroups: InstallGroup[] = [
  { comment: "Python library", commands: ["pip install --pre context-fabric"] },
  {
    comment: "MCP server for AI agents",
    commands: ["pip install --pre cfabric-mcp", "cfabric-mcp --corpus path/to/corpus"],
  },
];

/** Announces the 0.6 release candidate: its gains over 0.5 and how to try it. */
export function WhatsNew() {
  return (
    <section
      id="whats-new"
      className="scroll-mt-24 py-20 px-6 md:px-10 bg-[var(--color-bg)] border-t border-[var(--color-border)]"
    >
      <div className="max-w-[1000px] mx-auto">
        <div className="text-center mb-12">
          <div className="text-[0.8125rem] font-medium text-[var(--color-accent)] tracking-wide uppercase mb-4">
            New in 0.6 &middot; Release candidate
          </div>
          <h2 className="text-[2.25rem] md:text-[2.75rem] leading-[1.15] tracking-tight mb-4">
            A new Rust engine under the same API
          </h2>
          <p className="text-[1.0625rem] text-[var(--color-text-secondary)] max-w-[620px] mx-auto leading-relaxed">
            Your code and your results stay the same. Corpora load almost instantly, use a
            fraction of the memory, and search many times faster, with no required
            dependencies.
          </p>
        </div>

        <div className="grid grid-cols-3 gap-6 md:gap-10 max-w-[760px] mx-auto text-center">
          {releaseStats.map((stat) => (
            <div key={stat.label}>
              <div className="font-serif text-[2.25rem] md:text-[3rem] font-semibold text-[var(--color-accent)] leading-none mb-2">
                {stat.value}
              </div>
              <p className="text-[0.75rem] md:text-[0.875rem] font-medium text-[var(--color-text-secondary)] uppercase tracking-wider">
                {stat.label}
              </p>
            </div>
          ))}
        </div>
        <p className="text-center text-[0.8125rem] text-[var(--color-text-secondary)] mt-6">
          Compared to Context-Fabric 0.5.8 on the BHSA Hebrew Bible.
        </p>

        <div className="max-w-[560px] mx-auto mt-12">
          <InstallCommands groups={installGroups} />
          <p className="text-center text-[0.875rem] text-[var(--color-text-secondary)] mt-5">
            Something behaves differently from 0.5?{" "}
            <Link
              href="https://github.com/Context-Fabric/context-fabric/issues"
              className="underline underline-offset-2 hover:text-[var(--color-text)]"
            >
              Let us know
            </Link>{" "}
            &middot;{" "}
            <Link
              href={RELEASE_NOTES_URL}
              className="underline underline-offset-2 hover:text-[var(--color-text)]"
            >
              Release notes
            </Link>
          </p>
        </div>
      </div>
    </section>
  );
}
