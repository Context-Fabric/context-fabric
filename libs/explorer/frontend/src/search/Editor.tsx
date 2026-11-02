import { lazy, Suspense, type ComponentProps } from "react";
import type { QueryEditor as Real } from "./QueryEditor";

// The template editor (CodeMirror) is a large part of the code; it is loaded when a
// search form is first shown, so that reading doesn't wait for it.
const Loaded = lazy(() => import("./QueryEditor").then((m) => ({ default: m.QueryEditor })));

/** The template editor, loaded on first use; meanwhile the template as text. */
export function QueryEditor(props: ComponentProps<typeof Real>) {
  return (
    <Suspense
      fallback={
        <pre className="min-h-[5.5rem] whitespace-pre-wrap rounded border border-rule bg-vellum p-2 font-mono text-[14px] text-stone">
          {props.value}
        </pre>
      }
    >
      <Loaded {...props} />
    </Suspense>
  );
}
