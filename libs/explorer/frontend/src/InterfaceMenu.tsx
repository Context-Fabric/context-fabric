import { useCallback, useRef, useState } from "react";
import type { Common } from "./common";
import { guideUrl } from "./guide";
import { useHealth } from "./shared/health";
import { switchInterface } from "./switch";
import { canHide, hiddenUis, INTERFACES, isUi, rememberHiddenUis, type Ui } from "./ui";
import { useCloseOutside } from "./shared/popover";

interface Props {
  current: Ui;
  /** What the current interface shows, for the next one. */
  describe: () => Partial<Common> | Promise<Partial<Common>>;
}

/** The menu of interfaces: switch to another, keeping the passage and the corpora,
 * and choose which ones the menu offers. */
export function InterfaceMenu({ current, describe }: Props) {
  const [open, setOpen] = useState(false);
  const [choosing, setChoosing] = useState(false);
  const [busy, setBusy] = useState(false);
  const [failed, setFailed] = useState(false);
  const panel = useRef<HTMLDivElement>(null);
  // hidden here, or else as the server says (`ui: hide:` in browser.yaml)
  const { data: health } = useHealth();
  const [hidden, setHidden] = useState<Ui[] | null>(() => hiddenUis());
  const shownHidden = hidden ?? (health?.hiddenUis ?? []).filter(isUi);

  const close = useCallback(() => setOpen(false), []);
  useCloseOutside(open, close, panel);

  const ready = INTERFACES.filter((i) => i.ready);
  const offered = ready.filter((i) => i.id === current || !shownHidden.includes(i.id));
  const name = INTERFACES.find((i) => i.id === current)?.name ?? current;

  const go = async (target: Ui) => {
    if (target === current) return setOpen(false);
    setBusy(true);
    setFailed(false);
    try {
      await switchInterface(target, await describe());
    } catch {
      // the server didn't answer while finding the passage for the other
      // interface: say so, instead of a menu that does nothing
      setFailed(true);
    } finally {
      setBusy(false);
    }
  };

  const toggleHidden = (ui: Ui, hide: boolean) => {
    const next = hide ? [...shownHidden, ui] : shownHidden.filter((h) => h !== ui);
    setHidden(next);
    rememberHiddenUis(next);
  };

  return (
    <div className="relative" ref={panel}>
      <button
        type="button"
        aria-expanded={open}
        aria-haspopup="menu"
        disabled={busy}
        onClick={() => {
          setChoosing(false);
          setOpen((o) => !o);
        }}
        className="rounded border border-rule-strong px-2.5 py-0.5 text-[13px] hover:bg-woad-soft aria-expanded:bg-woad-soft"
        title="Choose an interface"
      >
        {name} <span aria-hidden>▾</span>
      </button>
      {open ? (
        <div
          role="menu"
          className="absolute end-0 top-full z-30 mt-1 w-[19rem] whitespace-normal rounded border border-rule-strong bg-raised py-1.5 text-[14px] shadow-lg"
        >
          {offered.map((i) => (
            <button
              key={i.id}
              type="button"
              role="menuitemradio"
              aria-checked={i.id === current}
              onClick={() => go(i.id)}
              className="flex w-full flex-col items-start px-3 py-1.5 text-left hover:bg-woad-soft aria-checked:bg-woad-soft/60"
            >
              <span className="font-semibold">
                {i.name}
                {i.id === current ? <span className="ms-1.5 text-[12px] font-normal text-stone">in use</span> : null}
              </span>
              <span className="text-[12px] leading-snug text-stone">{i.about}</span>
            </button>
          ))}
          {failed ? (
            <p role="alert" className="px-3 py-1 text-[13px] text-madder">
              Could not switch: the program did not answer. Try again.
            </p>
          ) : null}
          <div className="mt-1 border-t border-rule px-3 pt-1.5">
            {choosing ? (
              <fieldset className="flex flex-col gap-1 text-[13px]">
                <legend className="mb-1 text-[12px] text-stone">Offer in this menu</legend>
                {ready.map((i) => {
                  const shown = !shownHidden.includes(i.id);
                  return (
                    <label key={i.id} className="flex items-center gap-2">
                      <input
                        type="checkbox"
                        checked={shown}
                        disabled={shown && !canHide(i.id, shownHidden, current)}
                        onChange={(e) => toggleHidden(i.id, !e.target.checked)}
                        className="accent-[var(--woad)]"
                      />
                      {i.name}
                    </label>
                  );
                })}
              </fieldset>
            ) : (
              <button type="button" onClick={() => setChoosing(true)} className="text-[13px] text-woad hover:underline">
                Choose which to offer…
              </button>
            )}
          </div>
          <div className="mt-1.5 border-t border-rule px-3 pt-1.5">
            <a href={guideUrl(current)} role="menuitem" className="text-[13px] text-woad hover:underline">
              Guide to {name}
            </a>
          </div>
        </div>
      ) : null}
    </div>
  );
}
