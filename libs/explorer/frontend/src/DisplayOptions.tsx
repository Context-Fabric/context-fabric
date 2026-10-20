import { useEffect, useRef, useState } from "react";
import { FORMAT_NOTE, formatOptions, shownFormat } from "./formats";
import type { ViewSettings } from "./types";

/** The display menu: vocalized or unvocalized text (when the corpus offers
 * both, as the BHSA does) and what render trees show, kept per browser. */
export function DisplayOptions({
  settings,
  formats,
  defaultFormat,
  onChange,
}: {
  settings: ViewSettings;
  formats: string[];
  defaultFormat: string | null;
  onChange: (change: Partial<ViewSettings>) => void;
}) {
  const [open, setOpen] = useState(false);
  const panel = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (!open) return;
    const onDown = (e: MouseEvent) => {
      if (panel.current && !panel.current.contains(e.target as Node)) setOpen(false);
    };
    const onKey = (e: KeyboardEvent) => e.key === "Escape" && setOpen(false);
    window.addEventListener("mousedown", onDown);
    window.addEventListener("keydown", onKey);
    return () => {
      window.removeEventListener("mousedown", onDown);
      window.removeEventListener("keydown", onKey);
    };
  }, [open]);

  const options = formatOptions(formats);
  const toggles: [keyof ViewSettings, string][] = [
    ["showTypes", "Show object types"],
    ["standardFeatures", "Show each corpus' standard features"],
    ["hideTypes", "Hide minor object types"],
    ["showNodes", "Show node numbers"],
  ];

  return (
    <div className="relative" ref={panel}>
      <button
        type="button"
        aria-expanded={open}
        onClick={() => setOpen((o) => !o)}
        className="rounded px-2 py-1 text-[14px] hover:bg-woad-soft aria-expanded:bg-woad-soft sm:px-2.5"
      >
        Display
      </button>
      {open ? (
        <div className="absolute end-0 z-20 mt-1 flex w-72 flex-col gap-3 rounded border border-rule-strong bg-raised p-4 text-[14px] shadow-[0_8px_24px_-12px_rgba(31,43,77,0.35)]">
          {options.length ? (
            <label className="flex flex-col gap-1">
              <span className="text-[13px] text-stone">Text</span>
              <select
                value={shownFormat(settings.fmt, formats, defaultFormat) ?? ""}
                onChange={(e) => onChange({ fmt: e.target.value === defaultFormat ? null : e.target.value })}
                title={FORMAT_NOTE}
                className="rounded border border-rule-strong bg-vellum px-2 py-1"
              >
                {options.map((o) => (
                  <option key={o.value} value={o.value}>
                    {o.label}
                  </option>
                ))}
              </select>
              <span className="text-[12px] leading-snug text-stone">{FORMAT_NOTE}</span>
            </label>
          ) : null}
          {toggles.map(([key, label]) => (
            <label key={key} className="flex items-center gap-2">
              <input
                type="checkbox"
                checked={Boolean(settings[key])}
                onChange={(e) => onChange({ [key]: e.target.checked })}
                className="accent-[var(--woad)]"
              />
              {label}
            </label>
          ))}
        </div>
      ) : null}
    </div>
  );
}
