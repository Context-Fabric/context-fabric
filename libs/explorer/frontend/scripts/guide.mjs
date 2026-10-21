// Builds the user guide (libs/explorer/guide/*.md) into HTML pages: for the app
// (cfabric_browser/static/guide/, served at /guide/, so the guide works offline)
// and, with `--site DIR`, for caccht.github.io (DIR/explorer/), in the style of
// that site's own pages (data-use.html). Run by `npm run build`, after Vite,
// which empties the static folder first.
//
//     node scripts/guide.mjs                  # the app's pages
//     node scripts/guide.mjs --site ../site   # also the website's pages

import { mkdirSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { Marked } from "marked";

const here = dirname(fileURLToPath(import.meta.url));
export const SOURCE = resolve(here, "../../guide");
export const APP_OUT = resolve(here, "../../cfabric_browser/static/guide");

// the pages in reading order; index is the contents
export const PAGES = [
  "index",
  "getting-started",
  "datasets",
  "interfaces",
  "reading",
  "words",
  "searching",
  "concordance",
  "asking-claude",
  "adding-texts",
  "notes-and-citing",
  "problems",
];

/** A heading's id, as in links like `interfaces.md#panes`. */
export function slug(text) {
  return text
    .toLowerCase()
    .normalize("NFKD")
    .replace(/[̀-ͯ]/g, "")
    .replace(/<[^>]*>/g, "")
    .replace(/[^\w\s-]/g, "")
    .trim()
    .replace(/\s+/g, "-");
}

/** Links between pages are written to the Markdown files (`words.md#bdb`); the pages are HTML. */
export function pageLink(href) {
  const m = /^([\w-]+)\.md(#.*)?$/.exec(href);
  return m ? `${m[1]}.html${m[2] ?? ""}` : href;
}

function escape(text) {
  return text.replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;").replace(/"/g, "&quot;");
}

/** A page's title (its `#` heading), the rest as HTML, and the ids of its headings. */
export function render(markdown) {
  const ids = [];
  const marked = new Marked({
    renderer: {
      heading({ tokens, depth }) {
        const inner = this.parser.parseInline(tokens);
        const id = slug(inner);
        ids.push(id);
        return `<h${depth} id="${id}">${inner}</h${depth}>\n`;
      },
      link({ href, title, tokens }) {
        const inner = this.parser.parseInline(tokens);
        const target = pageLink(href);
        const external = /^https?:/.test(target) ? ' rel="noopener"' : "";
        return `<a href="${escape(target)}"${title ? ` title="${escape(title)}"` : ""}${external}>${inner}</a>`;
      },
    },
  });
  const tokens = marked.lexer(markdown);
  const first = tokens.findIndex((t) => t.type !== "space");
  let title = "";
  if (first >= 0 && tokens[first].type === "heading" && tokens[first].depth === 1) {
    title = tokens[first].text;
    tokens.splice(first, 1);
  }
  return { title, html: marked.parser(tokens), ids };
}

function readPages(source) {
  return PAGES.map((name) => ({ name, ...render(readFileSync(join(source, `${name}.md`), "utf8")) }));
}

function neighbours(pages, i) {
  const prev = i > 0 ? pages[i - 1] : null;
  const next = i < pages.length - 1 ? pages[i + 1] : null;
  const link = (p, rel, label) =>
    p ? `<a rel="${rel}" href="${p.name}.html"><span>${label}</span> ${escape(p.title)}</a>` : "<span></span>";
  // laid out here, so that it needs nothing from the website's stylesheet
  return `<nav class="guide-pn" style="display:flex;justify-content:space-between;gap:1rem;flex-wrap:wrap" aria-label="Previous and next">${link(prev, "prev", "Previous:")}${link(next, "next", "Next:")}</nav>`;
}

// the app's pages: the app's colours, its light or dark choice, a way back
const APP_STYLE = `
:root{--vellum:#faf9f6;--raised:#fff;--ink:#1c1b19;--stone:#6e6a63;--rule:#e4e1da;--woad:#34477a;--woad-soft:#e6e9f2;color-scheme:light}
[data-theme=dark]{--vellum:#15171d;--raised:#1c1f27;--ink:#ece9e2;--stone:#a19c92;--rule:#2c303b;--woad:#8ea2d8;--woad-soft:#232a3d;color-scheme:dark}
*{box-sizing:border-box}
body{margin:0;background:var(--vellum);color:var(--ink);font:16px/1.6 "Source Serif 4",Georgia,serif}
a{color:var(--woad)}
header{display:flex;align-items:center;gap:.6rem;padding:.5rem 1rem;border-bottom:1px solid var(--rule);background:var(--raised);font-size:15px}
header img{width:20px;height:20px}
header .name{font-weight:600}
header .back{margin-inline-start:auto;text-decoration:none}
.layout{display:grid;grid-template-columns:15rem minmax(0,44rem);gap:2.5rem;justify-content:center;padding:1.5rem 1rem 4rem}
.toc ol{list-style:none;margin:0;padding:0;font-size:14px;position:sticky;top:1rem}
.toc a{display:block;padding:.2rem .5rem;border-radius:4px;color:var(--ink);text-decoration:none}
.toc a:hover{background:var(--woad-soft)}
.toc a[aria-current=page]{background:var(--woad-soft);font-weight:600}
h1{font-size:2rem;line-height:1.2;margin:.2rem 0 1rem}
h2{font-size:1.35rem;margin:2rem 0 .5rem}
h3{font-size:1.1rem;margin:1.5rem 0 .4rem}
code{font:.9em "JetBrains Mono",ui-monospace,monospace;background:var(--woad-soft);padding:.05em .3em;border-radius:3px}
pre{background:var(--raised);border:1px solid var(--rule);padding:.8rem 1rem;overflow-x:auto;border-radius:4px}
pre code{background:none;padding:0}
table{border-collapse:collapse;margin:1rem 0;font-size:15px}
th,td{border:1px solid var(--rule);padding:.35rem .6rem;text-align:start;vertical-align:top}
img{max-width:100%}
.guide-pn{display:flex;justify-content:space-between;gap:1rem;margin-top:3rem;padding-top:1rem;border-top:1px solid var(--rule);font-size:15px}
.guide-pn span{color:var(--stone)}
@media (max-width:760px){.layout{grid-template-columns:minmax(0,1fr);gap:1rem}.toc ol{position:static}}
`;

// the theme the app uses; and where the reader came from, so that the way back
// returns to the same passage (the app keeps its state in the URL)
const APP_SCRIPT = `
try{var t=localStorage.getItem("cfabric-browser/theme");document.documentElement.dataset.theme=t==="light"||t==="dark"?t:matchMedia("(prefers-color-scheme: dark)").matches?"dark":"light"}catch(e){}
addEventListener("DOMContentLoaded",function(){var k="cfabric-browser/guide-from",b=document.querySelector("a.back");try{var r=document.referrer&&new URL(document.referrer);if(r&&r.origin===location.origin&&!r.pathname.startsWith("/guide"))sessionStorage.setItem(k,r.href);var f=sessionStorage.getItem(k);if(f&&b)b.href=f}catch(e){}});
`;

export function appPage(pages, i) {
  const page = pages[i];
  const toc = pages
    .map(
      (p) => `<li><a href="${p.name}.html"${p.name === page.name ? ' aria-current="page"' : ""}>${escape(p.title)}</a></li>`,
    )
    .join("");
  return `<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>${escape(page.title)}: CACCHT Explorer</title>
<link rel="icon" href="/favicon.svg">
<script>${APP_SCRIPT.trim()}</script>
<style>${APP_STYLE.trim()}</style>
</head>
<body>
<header><img src="/favicon.svg" alt=""><span class="name">CACCHT Explorer</span><span>Guide</span><a class="back" href="/">← Back to the Explorer</a></header>
<div class="layout">
<nav class="toc" aria-label="Contents"><ol>${toc}</ol></nav>
<main>
<h1>${escape(page.title)}</h1>
${page.html}${neighbours(pages, i)}
</main>
</div>
</body>
</html>
`;
}

// the website's pages, as its data-use.html: the same head, bar and footer
export function sitePage(pages, i) {
  const page = pages[i];
  const url = `https://caccht.github.io/explorer/${page.name === "index" ? "" : `${page.name}.html`}`;
  const toc = pages
    .map((p) => (p.name === page.name ? `<li>${escape(p.title)}</li>` : `<li><a href="${p.name}.html">${escape(p.title)}</a></li>`))
    .join("");
  const index = page.name === "index";
  const title = index ? `${escape(page.title)} — CACCHT` : `${escape(page.title)} — CACCHT Explorer`;
  return `<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>${title}</title>
<meta name="description" content="CACCHT Explorer, a program for reading, searching and comparing the Hebrew Bible, the Samaritan Pentateuch and the Dead Sea Scrolls: ${escape(page.title)}.">
<link rel="canonical" href="${url}">
<meta property="og:type" content="website">
<meta property="og:title" content="${title}">
<meta property="og:url" content="${url}">
<meta property="og:image" content="https://caccht.github.io/assets/logo.png">
<meta name="twitter:card" content="summary">
<link rel="icon" href="../assets/favicon.png" sizes="64x64" type="image/png">
<link rel="icon" href="../assets/favicon-32.png" sizes="32x32" type="image/png">
<link rel="apple-touch-icon" href="../assets/apple-touch-icon.png">
<link rel="preconnect" href="https://fonts.googleapis.com">
<link rel="preconnect" href="https://fonts.gstatic.com" crossorigin>
<link rel="stylesheet" href="https://fonts.googleapis.com/css2?family=Archivo:wdth,wght@85..112,400..800&family=Newsreader:ital,opsz,wght@0,6..72,300..600;1,6..72,300..500&family=IBM+Plex+Mono:wght@400;500&display=swap">
<link rel="stylesheet" href="../assets/css/site.css">
<style>main table{display:block;overflow-x:auto;border-collapse:collapse;margin:1.25rem 0}main th,main td{text-align:left;vertical-align:top;padding:.35rem 1rem .35rem 0;border-bottom:1px solid color-mix(in srgb,currentColor 18%,transparent)}main th{font-weight:600}</style>
</head>
<body>
<a class="skip" href="#main">Skip to content</a>

<header class="topbar">
  <div class="topbar__in">
    <a class="brand" href="../">
      <img class="brand__mark" src="../assets/logo.png" alt="" width="512" height="512">
      <span class="brand__name">CACCHT</span>
    </a>
    <nav class="nav" aria-label="Sections">
      <a href="../#corpora">Corpora</a>
      <a href="../#tools">Tools</a>
      <a href="../#method">Method</a>
      <a href="../#publications">Publications</a>
      <a href="../#team">Team</a>
      <a class="nav__gh" href="https://github.com/CACCHT">GitHub<span aria-hidden="true"> ↗</span></a>
    </nav>
  </div>
</header>

<main id="main">

<section class="band band--parch">
  <div class="wrap">
    <header class="shead shead--dark">
      <p class="label">CACCHT Explorer · Guide</p>
      <h1 class="shead__h">${escape(page.title)}</h1>
    </header>

    <div class="prose">
${page.html}${index ? "" : `
      <h2>Contents of the guide</h2>
      <ol>${toc}</ol>`}
${neighbours(pages, i)}
    </div>
  </div>
</section>

</main>

<footer class="foot">
  <div class="wrap foot__in">
    <img class="foot__mark" src="../assets/logo.png" alt="CACCHT" width="512" height="512">
    <div class="foot__t">
      <p class="foot__name">Creating Annotated Corpora of Classical Hebrew Texts</p>
      <p class="foot__p foot__contact">Contact: <a href="mailto:martijn.naaijer@uzh.ch">martijn.naaijer@uzh.ch</a></p>
      <p class="foot__p">Datasets are published under their own licenses; see each repository and the <a href="../data-use.html">data-use statement</a>. This site is maintained in <a href="https://github.com/CACCHT/caccht.github.io">caccht.github.io</a>.</p>
    </div>
    <nav class="foot__nav" aria-label="Elsewhere">
      <a href="https://github.com/CACCHT">GitHub<span aria-hidden="true"> ↗</span></a>
      <a href="https://annotation.github.io/text-fabric/tf/">Text-Fabric<span aria-hidden="true"> ↗</span></a>
      <a href="https://context-fabric.ai/">Context-Fabric<span aria-hidden="true"> ↗</span></a>
      <a href="https://etcbc.github.io/bhsa">BHSA<span aria-hidden="true"> ↗</span></a>
      <a href="https://etcbc.nl">ETCBC<span aria-hidden="true"> ↗</span></a>
    </nav>
  </div>
</footer>

<script src="../assets/js/site.js" defer></script>
</body>
</html>
`;
}

function write(dir, pages, page) {
  rmSync(dir, { recursive: true, force: true });
  mkdirSync(dir, { recursive: true });
  pages.forEach((p, i) => writeFileSync(join(dir, `${p.name}.html`), page(pages, i)));
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  const pages = readPages(SOURCE);
  write(APP_OUT, pages, appPage);
  console.log(`guide: ${pages.length} pages in ${APP_OUT}`);
  const at = process.argv.indexOf("--site");
  if (at > 0 && process.argv[at + 1]) {
    const out = join(resolve(process.argv[at + 1]), "explorer");
    write(out, pages, sitePage);
    console.log(`guide: ${pages.length} pages in ${out}`);
  }
}
