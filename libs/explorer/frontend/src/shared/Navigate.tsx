import { useQuery } from "@tanstack/react-query";
import { useEffect, useRef, useState } from "react";
import { api } from "../api";
import { displayRef } from "../display/Display";

/** Books, and the chapters of the book opened in the list. */
export function Navigate({
  corpus,
  current,
  onOpen,
  onClose,
}: {
  corpus: string;
  current: string | null;
  onOpen: (ref: string) => void;
  onClose: () => void;
}) {
  const panel = useRef<HTMLElement>(null);
  const currentBook = current ? current.split(" ")[0] : null;
  const [expanded, setExpanded] = useState<string | null>(currentBook);
  const books = useQuery({ queryKey: ["sections", corpus, null], queryFn: () => api.sections(corpus) });
  const chapters = useQuery({
    queryKey: ["sections", corpus, expanded],
    queryFn: () => api.sections(corpus, expanded!),
    enabled: expanded !== null,
  });

  useEffect(() => {
    panel.current?.querySelector<HTMLElement>("[aria-current]")?.scrollIntoView({ block: "center" });
    const onKey = (e: KeyboardEvent) => e.key === "Escape" && onClose();
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [onClose, books.data]);

  return (
    <>
      <div className="fixed inset-0 z-20 bg-[rgba(21,23,29,0.25)]" onClick={onClose} aria-hidden />
      <nav
        ref={panel}
        aria-label="Books"
        className="fixed inset-y-0 start-0 z-30 flex w-[17rem] max-w-[85vw] flex-col border-e border-rule bg-raised shadow-lg"
      >
        <header className="flex items-center border-b border-rule px-4 py-2.5">
          <h2 className="text-[15px] font-semibold">Navigate</h2>
          <button
            type="button"
            onClick={onClose}
            className="ms-auto rounded px-2 py-0.5 text-[13px] text-stone hover:bg-woad-soft hover:text-ink"
          >
            Close
          </button>
        </header>
        <ol className="min-h-0 flex-1 overflow-y-auto py-1">
          {(books.data?.children ?? []).map((b) => {
            const isOpen = expanded === b.ref;
            return (
              <li key={b.node}>
                <button
                  type="button"
                  aria-expanded={isOpen}
                  aria-current={b.ref === currentBook ? "true" : undefined}
                  onClick={() => setExpanded(isOpen ? null : b.ref)}
                  className="w-full px-4 py-1.5 text-left text-[14px] hover:bg-woad-soft aria-[current=true]:border-s-2 aria-[current=true]:border-woad aria-[current=true]:font-semibold"
                >
                  {displayRef(b.label)}
                </button>
                {isOpen ? (
                  <ol className="grid grid-cols-[repeat(auto-fill,minmax(2.4rem,1fr))] gap-1 px-4 pb-2 pt-1">
                    {(chapters.data?.children ?? []).map((c) => (
                      <li key={c.node}>
                        <button
                          type="button"
                          onClick={() => onOpen(c.ref)}
                          aria-current={c.ref === current ? "page" : undefined}
                          className="w-full rounded py-1 text-center text-[13px] tabular-nums hover:bg-woad-soft aria-[current=page]:bg-woad aria-[current=page]:text-vellum"
                        >
                          {c.label}
                        </button>
                      </li>
                    ))}
                    {chapters.isLoading ? <li className="col-span-full text-[13px] text-stone">…</li> : null}
                  </ol>
                ) : null}
              </li>
            );
          })}
        </ol>
      </nav>
    </>
  );
}
