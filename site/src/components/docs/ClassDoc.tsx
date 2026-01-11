import type { DocClass } from "@/types/docs";
import { MethodDoc } from "./MethodDoc";
import { isRenderableString, toRenderableString } from "@/lib/render";

interface ClassDocProps {
  cls: DocClass;
}

export function ClassDoc({ cls }: ClassDocProps) {
  const publicMethods = Object.values(cls.methods || {}).filter(
    (m) => !m.name.startsWith("_") || m.name === "__init__"
  );
  const hasAttributes = Object.keys(cls.attributes || {}).length > 0;

  const summary = cls.docstring?.summary;
  const description = cls.docstring?.description;

  return (
    <div id={cls.name} className="scroll-mt-20">
      <div className="flex items-baseline gap-3 mb-3">
        <span className="px-2 py-0.5 text-xs font-medium uppercase tracking-wider bg-[var(--color-accent)]/10 text-[var(--color-accent)] rounded">
          class
        </span>
        <h3 className="font-mono text-xl font-semibold text-[var(--color-text)]">
          {cls.name}
        </h3>
        {cls.bases && cls.bases.length > 0 && (
          <span className="text-sm text-[var(--color-text-secondary)]">
            ({cls.bases.join(", ")})
          </span>
        )}
      </div>

      {isRenderableString(summary) && (
        <p className="text-[var(--color-text-secondary)] mb-4 leading-relaxed">
          {summary}
        </p>
      )}

      {isRenderableString(description) && description !== summary && (
        <div className="text-[var(--color-text-secondary)] mb-4 text-sm whitespace-pre-wrap leading-relaxed">
          {description}
        </div>
      )}

      {hasAttributes && (
        <div className="mb-6">
          <h4 className="text-sm font-semibold text-[var(--color-text)] mb-3">
            Attributes
          </h4>
          <div className="bg-[var(--color-bg-alt)] border border-[var(--color-border)] rounded-lg overflow-hidden">
            <table className="w-full text-sm">
              <thead>
                <tr className="border-b border-[var(--color-border)]">
                  <th className="text-left px-4 py-2 font-medium text-[var(--color-text-secondary)]">
                    Name
                  </th>
                  <th className="text-left px-4 py-2 font-medium text-[var(--color-text-secondary)]">
                    Type
                  </th>
                  <th className="text-left px-4 py-2 font-medium text-[var(--color-text-secondary)]">
                    Description
                  </th>
                </tr>
              </thead>
              <tbody>
                {Object.values(cls.attributes).map((attr) => (
                  <tr
                    key={attr.name}
                    className="border-b border-[var(--color-border)] last:border-b-0"
                  >
                    <td className="px-4 py-2 font-mono text-[var(--color-accent)]">
                      {attr.name}
                    </td>
                    <td className="px-4 py-2 font-mono text-[var(--color-text-secondary)]">
                      {toRenderableString(attr.type) || "—"}
                    </td>
                    <td className="px-4 py-2 text-[var(--color-text-secondary)]">
                      {toRenderableString(attr.docstring?.summary) || "—"}
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        </div>
      )}

      {publicMethods.length > 0 && (
        <div>
          <h4 className="text-sm font-semibold text-[var(--color-text)] mb-3">
            Methods
          </h4>
          <div className="space-y-4">
            {publicMethods.map((method) => (
              <MethodDoc
                key={method.name}
                method={method}
                className={cls.name}
              />
            ))}
          </div>
        </div>
      )}
    </div>
  );
}
