import type { DocFunction } from "@/types/docs";
import { isRenderableString, toRenderableString } from "@/lib/render";

interface FunctionDocProps {
  fn: DocFunction;
}

export function FunctionDoc({ fn }: FunctionDocProps) {
  const params = fn.parameters?.filter((p) => p.name !== "self") || [];
  const summary = fn.docstring?.summary;
  const description = fn.docstring?.description;
  const returnType = toRenderableString(fn.returns?.type);

  return (
    <div
      id={fn.name}
      className="bg-[var(--color-bg-alt)] border border-[var(--color-border)] rounded-lg p-4 scroll-mt-20"
    >
      <div className="flex items-center gap-2 mb-2">
        <span className="px-1.5 py-0.5 text-[0.625rem] font-medium uppercase tracking-wider bg-blue-500/10 text-blue-600 dark:text-blue-400 rounded">
          function
        </span>
      </div>
      <div className="mb-2">
        <code className="text-sm">
          <span className="text-[var(--color-accent)] font-semibold">
            {fn.name}
          </span>
          <span className="text-[var(--color-text-secondary)]">
            {toRenderableString(fn.signature)}
          </span>
          {returnType && (
            <span className="text-[var(--color-text-secondary)]">
              {" "}
              → <span className="text-[var(--color-text)]">{returnType}</span>
            </span>
          )}
        </code>
      </div>

      {isRenderableString(summary) && (
        <p className="text-sm text-[var(--color-text-secondary)] mb-3">
          {summary}
        </p>
      )}

      {isRenderableString(description) && description !== summary && (
        <div className="text-sm text-[var(--color-text-secondary)] mb-3 whitespace-pre-wrap">
          {description}
        </div>
      )}

      {params.length > 0 && (
        <div className="mt-3 pt-3 border-t border-[var(--color-border)]">
          <h5 className="text-xs font-semibold uppercase tracking-wider text-[var(--color-text-secondary)] mb-2">
            Parameters
          </h5>
          <ul className="text-sm space-y-1.5">
            {params.map((param) => (
              <li key={param.name} className="flex items-start gap-2">
                <code className="text-[var(--color-accent)]">{param.name}</code>
                {isRenderableString(param.type) && (
                  <span className="text-[var(--color-text-secondary)]">
                    : {param.type}
                  </span>
                )}
                {isRenderableString(param.default) && (
                  <span className="text-[var(--color-text-secondary)]">
                    = {param.default}
                  </span>
                )}
              </li>
            ))}
          </ul>
        </div>
      )}
    </div>
  );
}
