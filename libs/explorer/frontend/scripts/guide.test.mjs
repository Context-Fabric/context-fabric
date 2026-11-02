import { existsSync, readdirSync, readFileSync } from "node:fs";
import { join } from "node:path";
import { describe, expect, it } from "vitest";
import { guideUrl } from "../src/guide";
import { INTERFACES } from "../src/ui";
import { appPage, pageLink, PAGES, render, sitePage, slug, SOURCE } from "./guide.mjs";
import { SHOTS } from "./screenshots.mjs";

const pages = PAGES.map((name) => ({ name, ...render(readFileSync(join(SOURCE, `${name}.md`), "utf8")) }));

describe("the guide's pages", () => {
  it("has a title on every page", () => {
    for (const p of pages) expect(p.title, p.name).not.toBe("");
  });

  it("links only to pages and headings that exist", () => {
    for (const p of pages) {
      for (const [, page, anchor] of p.html.matchAll(/href="([\w-]+)\.html(?:#([\w-]+))?"/g)) {
        const target = pages.find((q) => q.name === page);
        expect(target, `${p.name} links to ${page}.html`).toBeDefined();
        if (anchor) expect(target.ids, `${p.name} links to ${page}.html#${anchor}`).toContain(anchor);
      }
    }
  });

  it("shows only pictures that exist, and uses every one", () => {
    const used = new Set();
    for (const p of pages) {
      for (const [, file] of p.html.matchAll(/<img src="images\/([^"]+)"/g)) {
        expect(existsSync(join(SOURCE, "images", file)), `${p.name} shows ${file}`).toBe(true);
        used.add(file);
      }
    }
    for (const file of readdirSync(join(SOURCE, "images"))) expect(used, file).toContain(file);
    for (const shot of SHOTS) expect(used, shot.name).toContain(`${shot.name}.png`);
  });

  it("has a page and heading for every interface's Help", () => {
    for (const ui of INTERFACES) {
      const [, page, anchor] = /^\/guide\/([\w-]+)\.html(?:#([\w-]+))?$/.exec(guideUrl(ui.id));
      const target = pages.find((q) => q.name === page);
      expect(target, ui.id).toBeDefined();
      if (anchor) expect(target.ids, ui.id).toContain(anchor);
    }
  });

  it("builds both kinds of page", () => {
    const i = PAGES.indexOf("interfaces");
    expect(appPage(pages, i)).toContain('<h2 id="panes">Panes</h2>');
    expect(appPage(pages, i)).toContain('aria-current="page">The five ways of working</a>');
    expect(sitePage(pages, i)).toContain('href="../assets/css/site.css"');
    expect(sitePage(pages, i)).toContain('href="https://caccht.github.io/explorer/interfaces.html"');
  });
});

// Italics in the guide that are not the Explorer's own labels: other programs'
// buttons and menus, examples of requests and results, glosses, and emphasis.
const NOT_LABELS = new Set([
  // Windows, macOS, GitHub and Claude Desktop
  "Windows protected your PC", "More info", "Run anyway", "Properties", "Unblock", "About This Mac",
  "Chip Apple M…", "Applications", "System Settings → Privacy & Security", "Open Anyway", "Cmd+Shift+.",
  "View → Show → Hidden items", "Assets", "Settings → Extensions", "Install Extension…", "Advanced settings",
  "Extensions", "Settings → Developer → Edit Config", "Install", "Open",
  // the ChatGPT desktop app
  "Settings → MCP servers", "Add server", "Streamable HTTP", "Restart", "Go → Go to Folder…",
  // a release's name, requests to Claude, glosses and other examples
  "0.1.0-test.13", "find all cases of >LHJM/ in the BHSA", "clauses where the object comes before the verb",
  "Find all cases of >LHJM/ in the BHSA.", "All infinitives construct of HLK[.",
  "Clauses in Genesis where the subject comes before the verb.", "Lord", "lord", "cut down", "create", "speak",
  "word", "perish", "verses with both אלהים and ברא", "Searching… 812 results so far", "4Q7 = bhsa", "4Q7 other",
  "Subj", "Pred", "Objc", "Qal", "Niph.", "Pi.", "right before", "before", "petuhah", "setumah", "matres lectionis",
  "transliteration", "· bhsa",
  // the program's messages as the user sees them, with a number in them
  "Download (68 MB)", "Parallel ▾", "Stopped after 60s with … results. Narrow the template",
  "First … results (stopped at the limit)",
]);

/** The guide's italics, outside code: what it names in the program. */
function italics(markdown) {
  const text = markdown.replace(/```[\s\S]*?```/g, "").replace(/`[^`\n]*`/g, "code");
  return [...text.matchAll(/(?<![*\w])\*([^*\n]+?)\*(?![*\w])/g)].map((m) => m[1].trim());
}

/** The texts of the program's interface: its frontend, the guide's own pages, and the server's messages. */
function programText() {
  const roots = [
    [join(SOURCE, "../frontend/src"), /\.tsx?$/],
    [join(SOURCE, "../frontend/scripts"), /^guide\.mjs$/],
    [join(SOURCE, "../cfabric_browser"), /\.py$/],
    [join(SOURCE, "../packaging"), /\.py$/],
  ];
  let all = "";
  for (const [root, wanted] of roots) {
    for (const file of readdirSync(root, { recursive: true })) {
      const name = String(file).split(/[\\/]/).pop();
      if (wanted.test(name) && !/\.test\./.test(name)) all += readFileSync(join(root, String(file)), "utf8") + "\n";
    }
  }
  return all.replace(/\s+/g, " ");
}

/** Whether a label is in the program; `…` in the guide stands for a value, `→` separates menu and item. */
export function inProgram(label, text) {
  return label.split("→").every((part) => {
    const pieces = part.trim().split("…").map((p) => p.trim().replace(/[.*+?^${}()|[\]\\]/g, "\\$&"));
    return new RegExp(pieces.join("[\\s\\S]{0,80}?")).test(text);
  });
}

describe("the guide's names of things in the program", () => {
  it("names only labels that the program has", () => {
    const text = programText();
    const missing = [];
    for (const p of pages) {
      const markdown = readFileSync(join(SOURCE, `${p.name}.md`), "utf8");
      for (const label of italics(markdown)) {
        if (!NOT_LABELS.has(label) && !inProgram(label, text)) missing.push(`${p.name}: *${label}*`);
      }
    }
    expect(missing).toEqual([]);
  });

  it("matches labels with values and menus", () => {
    const text = "<span>In {books.length} books</span> title: 'Choose an interface' Display Text";
    expect(inProgram("In … books", text)).toBe(true);
    expect(inProgram("Display → Text", text)).toBe(true);
    expect(inProgram("Hide all", text)).toBe(false);
  });
});

describe("helpers", () => {
  it("makes ids from headings", () => {
    expect(slug("Switching and hiding")).toBe("switching-and-hiding");
    expect(slug("Notes, workspaces <em>and</em> citing")).toBe("notes-workspaces-and-citing");
    expect(slug("The JPS 1917")).toBe("the-jps-1917");
  });

  it("turns links to Markdown pages into links to HTML pages", () => {
    expect(pageLink("words.md#bdb")).toBe("words.html#bdb");
    expect(pageLink("datasets.md")).toBe("datasets.html");
    expect(pageLink("https://caccht.github.io/#corpora")).toBe("https://caccht.github.io/#corpora");
  });
});
