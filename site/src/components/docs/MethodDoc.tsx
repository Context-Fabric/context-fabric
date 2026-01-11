import type { DocFunction } from "@/types/docs";
import { isRenderableString, toRenderableString } from "@/lib/render";

interface MethodDocProps {
  method: DocFunction;
  className?: string;
}

export function MethodDoc({ method, className }: MethodDocProps) {
  const params = method.parameters?.filter((p) => p.name !== "self") || [];
  const anchorId = className ? `${className}.${method.name}` : method.name;
  const summary = method.docstring?.summary;
  const returnType = toRenderableString(method.returns?.type);

  return (
    <div
      id={anchorId}
      className="bg-[var(--color-bg-alt)] border border-[var(--color-border)] rounded-lg p-4 scroll-mt-20"
    >
      <div className="flex items-start gap-2 mb-2">
        <code className="text-sm flex-1">
          <span className="text-[var(--color-accent)] font-semibold">
            {method.name}
          </span>
          <span className="text-[var(--color-text-secondary)]">
            {toRenderableString(method.signature)}
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
