import { autocompletion, type CompletionContext, type CompletionResult } from "@codemirror/autocomplete";
import { defaultKeymap, history, historyKeymap, indentWithTab } from "@codemirror/commands";
import { HighlightStyle, StreamLanguage, syntaxHighlighting } from "@codemirror/language";
import { linter, lintGutter, type Diagnostic } from "@codemirror/lint";
import { EditorState } from "@codemirror/state";
import { EditorView, hoverTooltip, keymap, placeholder } from "@codemirror/view";
import { tags } from "@lezer/highlight";
import { useEffect, useRef } from "react";
import { api } from "../api";
import type { FeatureEntry } from "../types";

type FeatureInfo = Awaited<ReturnType<typeof api.feature>>;

// feature descriptions with their most frequent values, fetched once per corpus
const featureCache = new Map<string, Promise<FeatureInfo | null>>();

function featureInfo(corpus: string, name: string): Promise<FeatureInfo | null> {
  const key = `${corpus}/${name}`;
  let info = featureCache.get(key);
  if (!info) {
    info = api.feature(corpus, name).catch(() => null);
    featureCache.set(key, info);
  }
  return info;
}

// `sp=` or `sp=verb|` before the cursor: complete a value of `sp`
const VALUE_BEFORE = /([A-Za-z_][\w@]*)[=#]((?:[^\s|]*\|)*)([^\s|]*)$/;

// Search templates in Text-Fabric's query language: indentation for containment,
// `type feature=value` atoms, relation operators, names and quantifiers.

const QUANTIFIERS = /^\/(without|with|where|have|or|-)\//;
const RELATION = /^(<<|>>|<:|:>|<\d*:|:\d*>|=:|:=|::|<|>|==|&&|##|\|\||\[\[|\]\]|=\d*:|:\d*=|-[\w.]+>|<[\w.]+-|<[\w.]+>|\.[\w.]+[=#<>~][\w.]+\.)/;

const templateLanguage = StreamLanguage.define<{ atomSeen: boolean }>({
  startState: () => ({ atomSeen: false }),
  token(stream, state) {
    if (stream.sol()) state.atomSeen = false;
    if (stream.eatSpace()) return null;
    if (stream.peek() === "%") {
      stream.skipToEnd();
      return "comment";
    }
    if (stream.sol() || stream.string.slice(0, stream.pos).trim() === "") {
      if (stream.match(QUANTIFIERS)) return "keyword";
    }
    if (stream.match(RELATION)) return "operator";
    if (stream.match(/^[\w.]+:(?=\S)/)) return "labelName";
    if (!state.atomSeen && stream.match(/^[A-Za-z_][\w]*/)) {
      state.atomSeen = true;
      return "typeName";
    }
    if (stream.match(/^[A-Za-z_][\w@]*(?=[=#~<>*]|\s|$)/)) return "propertyName";
    if (stream.match(/^[=#~<>*]/)) return "operator";
    if (stream.match(/^[^\s]+/)) return "string";
    stream.next();
    return null;
  },
});

const highlight = HighlightStyle.define([
  { tag: tags.keyword, color: "var(--woad)", fontWeight: "600" },
  { tag: tags.typeName, color: "var(--ink)", fontWeight: "600" },
  { tag: tags.propertyName, color: "var(--woad)" },
  { tag: tags.operator, color: "var(--gold-deep)", fontWeight: "600" },
  { tag: tags.labelName, color: "var(--stone)", fontStyle: "italic" },
  { tag: tags.string, color: "var(--ink)" },
  { tag: tags.comment, color: "var(--stone)", fontStyle: "italic" },
]);

const theme = EditorView.theme({
  "&": {
    fontSize: "13.5px",
    backgroundColor: "var(--vellum-raised)",
    color: "var(--ink)",
    border: "1px solid var(--rule-strong)",
    borderRadius: "4px",
  },
  "&.cm-focused": { outline: "2px solid var(--woad)", outlineOffset: "1px" },
  ".cm-content": { fontFamily: "var(--font-mono)", padding: "8px 0", caretColor: "var(--ink)" },
  ".cm-scroller": { minHeight: "9.5em", maxHeight: "22em" },
  ".cm-gutters": { backgroundColor: "transparent", border: "none" },
  ".cm-placeholder": { color: "var(--stone)" },
  ".cm-tooltip": {
    backgroundColor: "var(--vellum-raised)",
    color: "var(--ink)",
    border: "1px solid var(--rule-strong)",
  },
  ".cm-tooltip-autocomplete ul li[aria-selected]": { backgroundColor: "var(--woad-soft)", color: "var(--ink)" },
  ".cm-selectionBackground, &.cm-focused .cm-selectionBackground": { backgroundColor: "var(--woad-soft) !important" },
});

interface Props {
  corpus: string;
  value: string;
  nodeTypes: string[];
  /** Number of nodes per type, shown when hovering a type name. */
  typeCounts?: Record<string, number>;
  features: FeatureEntry[];
  onChange: (value: string) => void;
  onRun: () => void;
}

/** A small tooltip: a name, a description and some values. Text only, never HTML. */
function tooltipBox(name: string, description: string, values: string[]): HTMLElement {
  const box = document.createElement("div");
  box.style.cssText = "max-width: 22rem; padding: 6px 9px; font-size: 13px; line-height: 1.4";
  const title = document.createElement("strong");
  title.textContent = name;
  const text = document.createElement("div");
  text.textContent = description.trim();
  box.append(title, text);
  if (values.length) {
    const list = document.createElement("div");
    list.style.cssText = "margin-top: 4px; color: var(--stone)";
    list.textContent = `Most frequent: ${values.join(", ")}`;
    box.append(list);
  }
  return box;
}

export function QueryEditor({ corpus, value, nodeTypes, typeCounts = {}, features, onChange, onRun }: Props) {
  const host = useRef<HTMLDivElement>(null);
  const view = useRef<EditorView | null>(null);
  const callbacks = useRef({ onChange, onRun });
  callbacks.current = { onChange, onRun };
  const vocabulary = useRef({ nodeTypes, features, typeCounts });
  vocabulary.current = { nodeTypes, features, typeCounts };

  useEffect(() => {
    if (!host.current) return;

    const complete = async (context: CompletionContext): Promise<CompletionResult | null> => {
      const line0 = context.state.doc.lineAt(context.pos);
      const valueMatch = VALUE_BEFORE.exec(line0.text.slice(0, context.pos - line0.from));
      if (valueMatch && vocabulary.current.features.some((f) => f.name === valueMatch[1])) {
        const info = await featureInfo(corpus, valueMatch[1]);
        const values = info?.sample_values ?? [];
        if (!values.length) return null;
        return {
          from: context.pos - valueMatch[3].length,
          options: values.map((v, i) => ({
            label: String(v.value),
            detail: v.count.toLocaleString("en-US"),
            type: "constant",
            boost: -i / values.length,
          })),
          validFor: /^[^\s|]*$/,
        };
      }
      const word = context.matchBefore(/[\w@]*/);
      if (!word || (word.from === word.to && !context.explicit)) return null;
      const line = context.state.doc.lineAt(context.pos);
      const before = line.text.slice(0, word.from - line.from);
      const isAtomStart = /^\s*([\w.]+:)?$/.test(before);
      const { nodeTypes: types, features: feats } = vocabulary.current;
      const options = isAtomStart
        ? types.map((t) => ({ label: t, type: "type" }))
        : feats
            .filter((f) => f.kind === "node" && f.name !== "otype")
            .map((f) => ({ label: f.name, type: "property", info: f.description || undefined }));
      return { from: word.from, options, validFor: /^[\w@]*$/ };
    };

    const check = linter(
      async (v): Promise<Diagnostic[]> => {
        const text = v.state.doc.toString();
        if (!text.trim()) return [];
        try {
          const result = await api.check(corpus, text);
          return result.problems.map((p) => {
            const lineNo = p.line && p.line <= v.state.doc.lines ? p.line : 1;
            const line = v.state.doc.line(lineNo);
            return {
              from: line.from,
              to: Math.max(line.from, line.to),
              severity: "error",
              message: p.message,
            };
          });
        } catch {
          return [];
        }
      },
      { delay: 400 },
    );

    const state = EditorState.create({
      doc: value,
      extensions: [
        history(),
        keymap.of([
          { key: "Mod-Enter", run: () => (callbacks.current.onRun(), true) },
          indentWithTab,
          ...defaultKeymap,
          ...historyKeymap,
        ]),
        templateLanguage,
        syntaxHighlighting(highlight),
        autocompletion({ override: [complete] }),
        hoverTooltip(async (view, pos) => {
          const line = view.state.doc.lineAt(pos);
          const text = line.text;
          let start = pos - line.from;
          let end = start;
          while (start > 0 && /[\w@]/.test(text[start - 1])) start--;
          while (end < text.length && /[\w@]/.test(text[end])) end++;
          if (start === end) return null;
          const word = text.slice(start, end);
          const { features: feats, typeCounts: counts } = vocabulary.current;
          let content: HTMLElement | null = null;
          if (word in counts) {
            content = tooltipBox(word, `${counts[word].toLocaleString("en-US")} in this corpus`, []);
          } else if (feats.some((f) => f.name === word)) {
            const info = await featureInfo(corpus, word);
            if (info) {
              const top = (info.sample_values ?? []).slice(0, 8).map((v) => `${v.value} (${v.count.toLocaleString("en-US")})`);
              const where = info.node_types?.length ? `On ${info.node_types.join(", ")}. ` : "";
              const how = info.unique_values ? `${info.unique_values.toLocaleString("en-US")} different values.` : "";
              content = tooltipBox(word, `${info.description || "No description."} ${where}${how}`, top);
            }
          }
          if (!content) return null;
          const box = content;
          return { pos: line.from + start, end: line.from + end, above: true, create: () => ({ dom: box }) };
        }),
        check,
        lintGutter(),
        theme,
        placeholder("clause\n  phrase function=Pred\n    word sp=verb"),
        EditorView.lineWrapping,
        EditorView.contentAttributes.of({ "aria-label": "Search template" }),
        EditorView.updateListener.of((u) => {
          if (u.docChanged) callbacks.current.onChange(u.state.doc.toString());
        }),
      ],
    });
    const editor = new EditorView({ state, parent: host.current });
    view.current = editor;
    return () => {
      editor.destroy();
      view.current = null;
    };
    // the editor is created once per corpus; later value changes come from typing
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [corpus]);

  // replace the text when it changes from outside (history, links)
  useEffect(() => {
    const editor = view.current;
    if (editor && editor.state.doc.toString() !== value) {
      editor.dispatch({ changes: { from: 0, to: editor.state.doc.length, insert: value } });
    }
  }, [value]);

  return <div ref={host} />;
}
