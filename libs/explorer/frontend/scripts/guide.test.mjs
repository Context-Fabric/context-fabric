import { readFileSync } from "node:fs";
import { join } from "node:path";
import { describe, expect, it } from "vitest";
import { guideUrl } from "../src/guide";
import { INTERFACES } from "../src/ui";
import { appPage, pageLink, PAGES, render, sitePage, slug, SOURCE } from "./guide.mjs";

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
