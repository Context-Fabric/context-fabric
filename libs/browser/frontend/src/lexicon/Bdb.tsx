import { useQuery } from "@tanstack/react-query";
import { Fragment, useMemo, useState, type ReactNode } from "react";
import { api } from "../api";
import { displayRef } from "../display/Display";
import type { BdbLink, CorpusCard } from "../types";
import { abbreviationPattern, linkNote, sefariaUrl, splitAbbreviations } from "./bdbText";

// BDB on the lexeme page: the entries a lexeme is linked to, and the text of
// one of them, with its verses as links that open the passage, its references
// to other entries, its abbreviations explained, its neighbours and its root
// family.

interface Props {
  corpus: CorpusCard;
  /** the lexeme's link; null when BDB hasn't been added */
  link: BdbLink | null;
  /** the Hebrew font */
  script: string;
  onOpenPassage: (ref: string) => void;
  onLex: (node: number) => void;
}

export function BdbSection({ corpus, link, script, onOpenPassage, onLex }: Props) {
  const [chosen, setChosen] = useState<string | null>(null);
  if (link === null) {
    return (
      <p className="p-4 text-[14px] text-stone">
        BDB, the Hebrew and English lexicon of Brown, Driver and Briggs (1906), is not on this computer. It can be added
        from the library, under <em>Dictionaries</em>.
      </p>
    );
  }
  if (link.entries.length === 0) return <p className="p-4 text-[14px] text-stone">This lexeme is not linked to BDB.</p>;
  const linked = link.entries.map((e) => e.id);
  const shown = chosen ?? linked[0];
  const note = linkNote(link);
  return (
    <div className="flex flex-col gap-3 p-4">
      {linked.length > 1 || note ? (
        <div className="flex flex-col gap-1 text-[13px]">
          {linked.length > 1 ? (
            <div className="flex flex-wrap items-baseline gap-2">
              <span className="text-stone">Linked to</span>
              {link.entries.map((e, i) => (
                <button
                  key={e.id}
                  type="button"
                  onClick={() => setChosen(e.id)}
                  aria-pressed={e.id === shown}
                  className="rounded border border-rule px-2 py-0.5 hover:border-woad aria-pressed:border-woad aria-pressed:text-woad"
                  title={i === 0 ? "The main entry" : "Also linked: some of its occurrences lead here"}
                >
                  <span dir="rtl" className={script}>
                    {e.headword}
                  </span>{" "}
                  {e.short}
                </button>
              ))}
            </div>
          ) : null}
          {note ? <p className="text-stone">{note}</p> : null}
        </div>
      ) : null}
      <Article
        key={shown}
        id={shown}
        corpus={corpus}
        script={script}
        onEntry={setChosen}
        onOpenPassage={onOpenPassage}
        onLex={onLex}
      />
    </div>
  );
}

interface ArticleProps {
  id: string;
  corpus: CorpusCard;
  script: string;
  onEntry: (id: string) => void;
  onOpenPassage: (ref: string) => void;
  onLex: (node: number) => void;
}

function Article({ id, corpus, script, onEntry, onOpenPassage, onLex }: ArticleProps) {
  const { data: article, error } = useQuery({
    queryKey: ["bdb-entry", id, corpus.name],
    queryFn: () => api.bdbEntry(id, corpus.name),
    staleTime: Infinity,
  });
  const { data: dictionary } = useQuery({
    queryKey: ["dictionary", "bdb"],
    queryFn: () => api.dictionary("bdb"),
    staleTime: Infinity,
  });
  const abbreviations = useMemo(() => dictionary?.abbreviations ?? {}, [dictionary]);
  const pattern = useMemo(() => abbreviationPattern(abbreviations), [abbreviations]);
  const body = useMemo(
    () =>
      article
        ? render(article.html, {
            script,
            passages: article.passages,
            abbreviations,
            pattern,
            onOpenPassage,
            onEntry,
          })
        : null,
    [article, script, abbreviations, pattern, onOpenPassage, onEntry],
  );
  if (error) return <p className="text-madder">Could not load the entry: {String((error as Error).message)}</p>;
  if (!article) return <p className="text-stone">Loading…</p>;

  return (
    <article className="flex flex-col gap-3">
      {article.incomplete ? (
        <p className="rounded border border-rule bg-raised px-3 py-2 text-[13px] text-stone">
          {article.html
            ? "Sefaria's text of this entry stops short; the short definition is Open Scriptures'."
            : "Sefaria doesn't have this entry; shown is Open Scriptures' short definition."}
          {article.defs.length ? ` ${article.pos ? `${article.pos}. ` : ""}${article.defs.join(", ")}.` : ""}
        </p>
      ) : null}
      {article.html ? <div className="bdb max-w-[48rem] text-[15px] leading-relaxed">{body}</div> : (
        <p dir="rtl" className={`${script} text-[20px]`}>
          {article.headword}
        </p>
      )}
      <footer className="flex flex-col gap-2 border-t border-rule pt-2 text-[13px]">
        <nav className="flex flex-wrap gap-x-4 gap-y-1">
          {article.previous ? (
            <button type="button" onClick={() => onEntry(article.previous!.id)} className="text-woad hover:underline">
              ← <span className={script}>{article.previous.headword}</span>
            </button>
          ) : null}
          {article.next ? (
            <button type="button" onClick={() => onEntry(article.next!.id)} className="text-woad hover:underline">
              <span className={script}>{article.next.headword}</span> →
            </button>
          ) : null}
          <span className="text-stone">in BDB</span>
        </nav>
        {article.family.length ? (
          <section>
            <h3 className="font-semibold">Root family</h3>
            <ul className="mt-1 grid grid-cols-[auto_minmax(0,1fr)] gap-x-3 gap-y-0.5">
              {article.family.map((m) => (
                <li key={m.id} className="contents">
                  <button
                    type="button"
                    onClick={() => onEntry(m.id)}
                    aria-current={m.id === article.id}
                    className="text-right hover:text-woad aria-[current=true]:font-semibold"
                  >
                    <span dir="rtl" className={script}>
                      {m.headword}
                    </span>
                  </button>
                  <span className="truncate">
                    {[m.pos, m.short].filter(Boolean).join(" ")}
                    {m.lexemes.map((x) => (
                      <button
                        key={x.node}
                        type="button"
                        onClick={() => onLex(x.node)}
                        className="ms-2 text-woad hover:underline"
                        title={`The lexeme ${x.lex} in ${corpus.title}`}
                      >
                        {x.gloss ?? x.lex}
                      </button>
                    ))}
                  </span>
                </li>
              ))}
            </ul>
          </section>
        ) : null}
        <p className="text-stone">
          {article.strong.length ? `Strong's ${article.strong.join(", ")} · ` : ""}
          {article.twot.length ? `TWOT ${article.twot.join(", ")} · ` : ""}
          Brown, Driver, Briggs, <em>A Hebrew and English Lexicon of the Old Testament</em> (1906)
          {article.ref ? (
            <>
              {" · "}
              <a href={sefariaUrl(article.ref)} target="_blank" rel="noreferrer" className="text-woad hover:underline">
                On Sefaria
              </a>
            </>
          ) : null}
        </p>
      </footer>
    </article>
  );
}

interface RenderContext {
  script: string;
  passages: Record<string, string>;
  abbreviations: Record<string, string>;
  pattern: RegExp | null;
  onOpenPassage: (ref: string) => void;
  onEntry: (id: string) => void;
}

// the tags the builder keeps; anything else is shown as its text
const INLINE = new Set(["sub", "sup", "strong", "em"]);

/** The cleaned HTML of an entry as React elements. */
function render(html: string, ctx: RenderContext): ReactNode {
  const doc = new DOMParser().parseFromString(`<body>${html}</body>`, "text/html");
  return Array.from(doc.body.childNodes).map((n, i) => node(n, ctx, false, i));
}

function node(n: Node, ctx: RenderContext, inLink: boolean, key: number): ReactNode {
  if (n.nodeType === Node.TEXT_NODE) {
    const text = n.textContent ?? "";
    if (inLink) return text;
    return (
      <Fragment key={key}>
        {splitAbbreviations(text, ctx.pattern, ctx.abbreviations).map((p, i) =>
          typeof p === "string" ? (
            p
          ) : (
            <abbr key={i} title={p.meaning} className="cursor-help no-underline decoration-dotted hover:underline">
              {p.abbr}
            </abbr>
          ),
        )}
      </Fragment>
    );
  }
  if (n.nodeType !== Node.ELEMENT_NODE) return null;
  const el = n as Element;
  const tag = el.tagName.toLowerCase();
  const children = (link = inLink) => Array.from(el.childNodes).map((c, i) => node(c, ctx, link, i));
  if (tag === "br") return <br key={key} />;
  if (INLINE.has(tag)) {
    const Tag = tag as "sub" | "sup" | "strong" | "em";
    return <Tag key={key}>{children()}</Tag>;
  }
  if (tag === "big") {
    return (
      <span key={key} className="text-[1.25em]">
        {children()}
      </span>
    );
  }
  if (tag === "span") {
    const rtl = el.getAttribute("dir") === "rtl";
    return (
      <span key={key} dir={rtl ? "rtl" : undefined} className={rtl ? ctx.script : undefined}>
        {children()}
      </span>
    );
  }
  if (tag === "a") {
    const osis = el.getAttribute("data-osis");
    const entry = el.getAttribute("data-entry");
    const ref = osis ? ctx.passages[osis] : undefined;
    if (ref) {
      return (
        <button key={key} type="button" onClick={() => ctx.onOpenPassage(ref)} className="text-woad hover:underline" title={`Open ${displayRef(ref)}`}>
          {children(true)}
        </button>
      );
    }
    if (entry) {
      return (
        <button key={key} type="button" onClick={() => ctx.onEntry(entry)} className="text-woad hover:underline" title="Show this entry">
          {children(true)}
        </button>
      );
    }
    return <span key={key}>{children(true)}</span>;
  }
  return <Fragment key={key}>{children()}</Fragment>;
}
