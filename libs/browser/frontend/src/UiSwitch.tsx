import { useState } from "react";
import type { Ui } from "./ui";

const LABELS: Record<Ui, { label: string; title: string }> = {
  classic: { label: "Panes", title: "Panes side by side, each with its own corpus, passage and view" },
  parallel: { label: "Parallel", title: "A column per corpus and a row per verse, after Parabible" },
};

/** Switches between the two interfaces, keeping the passage and the corpora. */
export function UiSwitch({ current, onSwitch }: { current: Ui; onSwitch: () => Promise<void> }) {
  const [busy, setBusy] = useState(false);
  return (
    <div className="flex rounded border border-rule-strong text-[13px]" role="group" aria-label="Interface">
      {(["classic", "parallel"] as const).map((ui) => (
        <button
          key={ui}
          type="button"
          aria-pressed={current === ui}
          disabled={busy}
          title={LABELS[ui].title}
          onClick={async () => {
            if (ui === current) return;
            setBusy(true);
            try {
              await onSwitch();
            } finally {
              setBusy(false);
            }
          }}
          className="px-2.5 py-0.5 aria-pressed:bg-ink aria-pressed:text-vellum"
        >
          {LABELS[ui].label}
        </button>
      ))}
    </div>
  );
}
