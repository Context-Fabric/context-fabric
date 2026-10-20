import { createContext, useContext, type CSSProperties, type ReactNode } from "react";
import type { DiffKind, DisplayConfig, FeatureValue, Fragment, RenderTree, ViewSettings } from "../types";
import "./display.css";

// Draws render trees from cfabric.display. Pretty displays nest the fragments as
// frames whose edges say what they are: a solid edge is where the node really
// starts or ends, a dashed edge borders a gap in the node, and an open edge
// means the node continues beyond this fragment.

interface DisplayContextValue {
  config: DisplayConfig;
  settings: ViewSettings;
  onInspect?: (node: number) => void;
  /** A word is hovered over (null: no longer), for the card with its essentials. */
  onHover?: (node: number | null, target?: Element) => void;
  /** Words that differ from the text in another pane. */
  marks?: Map<number, DiffKind>;
  /** Per marked word: what it differs from. */
  markTitles?: Map<number, string>;
}

const DisplayContext = createContext<DisplayContextValue | null>(null);

export function DisplayProvider({
  config,
  settings,
  onInspect,
  onHover,
  marks,
  markTitles,
  children,
}: DisplayContextValue & { children: ReactNode }) {
  return (
    <DisplayContext.Provider value={{ config, settings, onInspect, onHover, marks, markTitles }}>
      {children}
    </DisplayContext.Provider>
  );
}

function useDisplay(): DisplayContextValue {
  const value = useContext(DisplayContext);
  if (!value) throw new Error("Display components need a DisplayProvider");
  return value;
}

/** A section reference for people: TF names use underscores for spaces (1_Samuel). */
export function displayRef(ref: string): string {
  return ref.replace(/_/g, " ");
}

/** CSS class for corpus text in a given text format. */
export function scriptClass(config: DisplayConfig, direction: "rtl" | "ltr"): string {
  // only formats in the original script get the script's font
  const code = config.languageCode;
  if (!code || (direction === "ltr" && config.direction === "rtl")) return "script";
  return `script script-${code}`;
}

function highlightStyle(color: string | undefined): CSSProperties | undefined {
  if (!color) return undefined;
  return { background: color, boxShadow: "none" };
}

function diffClass(marks: Map<number, DiffKind> | undefined, node: number): string {
  const kind = marks?.get(node);
  return kind ? `diff diff-${kind}` : "";
}

/** Mouse and focus handlers that report a word to `onHover`. */
function hoverProps(onHover: DisplayContextValue["onHover"], node: number) {
  if (!onHover) return {};
  const enter = (e: { currentTarget: Element }) => onHover(node, e.currentTarget);
  const leave = () => onHover(null);
  return { onMouseEnter: enter, onMouseLeave: leave, onFocus: enter, onBlur: leave };
}

function highlightClass(color: string | undefined): string {
  if (color === undefined) return "";
  return color ? "hl-custom" : "hl";
}

// plain

function isBaseType(config: DisplayConfig, type: string): boolean {
  return type !== config.slotType && config.baseTypes.includes(type);
}

// Where slots are below word level (signs in SP and DSS), the base type (word) is
// the unit to click: its signs are drawn inside it and are not inspected one by one.
function PlainFragment({ fragment, inBase = false }: { fragment: Fragment; inBase?: boolean }) {
  const { config, onInspect, onHover, marks, markTitles } = useDisplay();
  const isText = fragment.text !== undefined;
  const isBase = !inBase && !isText && isBaseType(config, fragment.type);
  const cls = `${highlightClass(fragment.highlight)} ${fragment.dim ? "dim" : ""}`.trim();
  const content = isText
    ? fragment.text
    : (fragment.children ?? []).map((child, i) => (
        <PlainFragment key={i} fragment={child} inBase={inBase || isBase} />
      ));

  if (((isText && !inBase) || isBase) && onInspect) {
    return (
      <span
        className={`plain-word ${cls} ${diffClass(marks, fragment.node)}`}
        style={highlightStyle(fragment.highlight)}
        // the hover card shows what the word differs from
        title={onHover ? undefined : markTitles?.get(fragment.node)}
        onClick={() => onInspect(fragment.node)}
        {...hoverProps(onHover, fragment.node)}
      >
        {content}
      </span>
    );
  }
  if (!cls) return <>{content}</>;
  return (
    <span className={cls} style={highlightStyle(fragment.highlight)}>
      {content}
    </span>
  );
}

export function Plain({ tree }: { tree: RenderTree }) {
  const { config } = useDisplay();
  const content = tree.fragments.some((f) => f.text !== undefined || f.children?.length)
    ? tree.fragments.map((f, i) => <PlainFragment key={i} fragment={f} />)
    : tree.heading;
  return (
    <span dir={tree.direction} className={`plain ${scriptClass(config, tree.direction)}`}>
      {content}
    </span>
  );
}

// pretty

function visibleFeatures(features: FeatureValue[] | undefined, settings: ViewSettings): FeatureValue[] {
  if (!features) return [];
  return features.filter((f) => f.kind === "extra" || f.extra || settings.standardFeatures);
}

function Features({ features }: { features: FeatureValue[] }) {
  if (!features.length) return null;
  return (
    <dl className="features" dir="ltr">
      {features.map((f) => (
        <div key={f.name} className={f.extra ? "feature feature-asked" : "feature"} title={f.on ? `${f.name} of the ${f.on}` : f.name}>
          {f.kind === "bare" ? null : <dt>{f.name}</dt>}
          <dd>{String(f.value)}</dd>
        </div>
      ))}
    </dl>
  );
}

function PrettyFragment({ fragment, direction }: { fragment: Fragment; direction: "rtl" | "ltr" }) {
  const { config, settings, onInspect, onHover, marks, markTitles } = useDisplay();
  const typeDisplay = config.types[fragment.type];
  const level = typeDisplay?.level ?? 1;
  const flow = typeDisplay?.flow ?? "hor";
  const children = fragment.children ?? [];
  // base fragments (e.g. a phrase shown as one block) also carry text, but keep their label
  const isText = fragment.text !== undefined && !fragment.base;
  const features = visibleFeatures(fragment.features, settings);
  const label = fragment.heading ?? fragment.label;
  const highlighted = fragment.highlight !== undefined;

  const className = [
    "frag",
    `level-${level}`,
    `start-${fragment.start}`,
    `end-${fragment.end}`,
    isText ? "frag-text" : "",
    highlighted ? "frag-hl" : "",
  ].join(" ");

  const style: CSSProperties | undefined = fragment.highlight
    ? ({ "--hl": fragment.highlight } as CSSProperties)
    : undefined;

  const showLabel = !isText && (settings.showTypes || label || settings.showNodes);

  return (
    <div className={className} style={style}>
      {showLabel ? (
        <div className="frag-label" dir="ltr">
          {settings.showTypes ? <span className="frag-type">{fragment.type}</span> : null}
          {label ? <span className="frag-name">{label}</span> : null}
          {settings.showNodes ? (
            <button type="button" className="frag-node" onClick={() => onInspect?.(fragment.node)}>
              {fragment.node}
            </button>
          ) : null}
        </div>
      ) : null}
      {isText ? (
        <button
          type="button"
          className={`frag-word ${scriptClass(config, direction)} ${diffClass(marks, fragment.node)} ${fragment.dim ? "dim" : ""}`}
          dir={direction}
          onClick={() => onInspect?.(fragment.node)}
          title={
            onHover
              ? undefined
              : (markTitles?.get(fragment.node) ?? (settings.showNodes ? undefined : `${fragment.type} ${fragment.node}`))
          }
          {...hoverProps(onHover, fragment.node)}
        >
          {fragment.text}
        </button>
      ) : null}
      {isText && settings.showNodes ? (
        <span className="frag-node frag-node-slot">{fragment.node}</span>
      ) : null}
      {!isText && fragment.base && fragment.text ? (
        <button
          type="button"
          className={`frag-word frag-base ${scriptClass(config, direction)} ${diffClass(marks, fragment.node)} ${fragment.dim ? "dim" : ""}`}
          dir={direction}
          onClick={() => onInspect?.(fragment.node)}
          title={onHover || settings.showNodes ? undefined : `${fragment.type} ${fragment.node}`}
          {...hoverProps(onHover, fragment.node)}
        >
          {fragment.text}
        </button>
      ) : null}
      <Features features={features} />
      {children.length ? (
        <div className={`frag-children flow-${flow}`} dir={direction}>
          {children.map((child, i) => (
            <PrettyFragment key={i} fragment={child} direction={direction} />
          ))}
        </div>
      ) : null}
    </div>
  );
}

export function Pretty({ tree }: { tree: RenderTree }) {
  return (
    <div className="pretty" dir={tree.direction}>
      {tree.fragments.map((f, i) => (
        <PrettyFragment key={i} fragment={f} direction={tree.direction} />
      ))}
    </div>
  );
}

export function Tree({ tree }: { tree: RenderTree }) {
  return tree.mode === "plain" ? <Plain tree={tree} /> : <Pretty tree={tree} />;
}
