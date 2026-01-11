import type { DocModule } from "@/types/docs";
import { ClassDoc } from "./ClassDoc";
import { FunctionDoc } from "./FunctionDoc";
import { DocstringContent } from "./DocstringContent";
import { isRenderableString } from "@/lib/render";
import Link from "next/link";

interface ModuleDocProps {
  module: DocModule;
  basePath: string;
}

export function ModuleDoc({ module, basePath }: ModuleDocProps) {
  const hasClasses = Object.keys(module.classes || {}).length > 0;
  const hasFunctions = Object.keys(module.functions || {}).length > 0;
  const hasSubmodules = Object.keys(module.modules || {}).length > 0;

  const summary = module.docstring?.summary;
  const rawDescription = module.docstring?.description;
  // Remove duplicate summary from beginning of description
  const description = rawDescription && summary && rawDescription.startsWith(summary)
    ? rawDescription.slice(summary.length).trim()
    : rawDescription;

  return (
    <article className="max-w-4xl">
      <header className="mb-8">
        <h1 className="font-serif text-[2.5rem] font-semibold text-[var(--color-text)] mb-4">
          {module.name}
        </h1>
        {isRenderableString(summary) && (
          <p className="text-[1.125rem] text-[var(--color-text-secondary)] leading-relaxed">
            {summary}
          </p>
        )}
        {isRenderableString(description) && description !== summary && (
          <DocstringContent content={description} className="mt-4" />
        )}
      </header>

      {hasSubmodules && (
        <section className="mb-12">
          <h2 className="text-xl font-semibold text-[var(--color-text)] mb-4 pb-2 border-b border-[var(--color-border)]">
            Submodules
          </h2>
          <ul className="space-y-2">
            {Object.entries(module.modules).map(([name, submod]) => {
              const subSummary = submod.docstring?.summary;
              return (
                <li key={name}>
                  <Link
                    href={`${basePath}/${name}`}
                    className="group flex items-start gap-3 p-3 rounded-lg hover:bg-[var(--color-bg-alt)] transition-colors"
                  >
                    <span className="font-mono text-[var(--color-accent)] font-medium group-hover:underline">
                      {name}
                    </span>
                    {isRenderableString(subSummary) && (
                      <span className="text-[var(--color-text-secondary)] text-sm">
                        {subSummary}
                      </span>
                    )}
                  </Link>
                </li>
              );
            })}
          </ul>
        </section>
      )}

      {hasClasses && (
        <section className="mb-12">
          <h2 className="text-xl font-semibold text-[var(--color-text)] mb-6 pb-2 border-b border-[var(--color-border)]">
            Classes
          </h2>
          <div className="space-y-8">
            {Object.values(module.classes).map((cls) => (
              <ClassDoc key={cls.name} cls={cls} />
            ))}
          </div>
        </section>
      )}

      {hasFunctions && (
        <section className="mb-12">
          <h2 className="text-xl font-semibold text-[var(--color-text)] mb-6 pb-2 border-b border-[var(--color-border)]">
            Functions
          </h2>
          <div className="space-y-6">
            {Object.values(module.functions)
              .filter((fn) => !fn.name.startsWith("_"))
              .map((fn) => (
                <FunctionDoc key={fn.name} fn={fn} />
              ))}
          </div>
        </section>
      )}
    </article>
  );
}
