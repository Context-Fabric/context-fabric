// Takes the guide's screenshots (libs/explorer/guide/images/) from a running
// Explorer, so that they can be redone for each release instead of by hand.
// Start the Explorer on the real corpora first, with an empty home folder apart
// from the corpora and BDB, so that the pictures show what users see:
//
//     cfabric-browser bhsa "sp=DT-UCPH/sp@7.2.4" "dss=ETCBC/dss@2.0.1" \
//         "jps1917=Context-Fabric/jps1917" --port 8799 --no-open
//     node scripts/screenshots.mjs http://127.0.0.1:8799 [name ...]
//
// The shot of a proposed search needs the browser's secret (in
// .cfabric/running.json of its home folder) in CFABRIC_SECRET.
//
// Uses the Chrome installed on this computer (playwright-core downloads no
// browser of its own).

import { mkdirSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { chromium } from "playwright-core";

const here = dirname(fileURLToPath(import.meta.url));
const OUT = resolve(here, "../../guide/images");
const SIZE = { width: 1280, height: 760 };

// each shot: a URL, what to wait for, what to do before taking it, and the theme
export const SHOTS = [
  {
    name: "parallel",
    url: "/Genesis/1?texts=bhsa,sp,dss",
    ready: "text=4Q2",
  },
  {
    name: "panes",
    url: "/?c0=bhsa&ref0=Genesis+1&c1=dss&by1=books&sync=1",
    ready: "text=4Q2",
  },
  {
    name: "reader",
    url: "/read/bhsa/Genesis/1?v=3&panel=parallels&texts=sp,jps1917",
    ready: "text=Parallels",
  },
  {
    name: "reader-dark",
    url: "/read/bhsa/Genesis/1?mode=running",
    ready: "text=Running text",
    theme: "dark",
  },
  {
    name: "interlinear",
    url: "/interlinear/bhsa/Genesis/1?under=sp&struct=1",
    ready: "text=Lines",
  },
  {
    name: "menu",
    url: "/Genesis/1?texts=bhsa,sp,dss",
    ready: "text=4Q2",
    act: (page) => page.getByTitle("Choose an interface").click(),
    clip: { x: 760, y: 0, width: 520, height: 420 },
  },
  {
    name: "differences",
    url: "/Exodus/20?texts=bhsa,sp,dss&diff=1&v=17",
    ready: "text=words differ",
  },
  {
    name: "navigate",
    url: "/Genesis/1?texts=bhsa,sp,dss",
    ready: "text=4Q2",
    act: async (page) => {
      await page.getByRole("button", { name: "Choose a book and chapter" }).click();
      await page.getByRole("button", { name: "Exodus", exact: true }).click();
    },
  },
  {
    name: "hover",
    url: "/Genesis/1?texts=bhsa,sp,dss",
    ready: "text=4Q2",
    act: async (page) => {
      await page.locator(".plain-word").nth(2).hover();
      await page.waitForSelector("text=Click for all features");
    },
    clip: { x: 0, y: 40, width: 760, height: 420 },
  },
  {
    name: "inspector",
    url: "/Genesis/1?texts=bhsa,sp,dss",
    ready: "text=4Q2",
    act: async (page) => {
      await page.locator(".plain-word").nth(2).click();
      await page.waitForSelector("text=Add as search term");
      await page.mouse.move(1270, 750);
    },
  },
  {
    name: "lexicon",
    url: "/Genesis/1?texts=bhsa,sp,dss",
    ready: "text=4Q2",
    act: async (page) => {
      await page.getByRole("button", { name: "Lexicon" }).click();
      await page.getByLabel("Find a lexeme").fill("create");
      await page.getByText("BR>[").first().click();
      await page.waitForSelector("text=occurrences in");
    },
  },
  {
    name: "bdb",
    url: "/Genesis/1?texts=bhsa,sp,dss",
    ready: "text=4Q2",
    act: async (page) => {
      await page.getByRole("button", { name: "Lexicon" }).click();
      await page.getByLabel("Find a lexeme").fill("create");
      await page.getByText("BR>[").first().click();
      await page.getByRole("tab", { name: "BDB" }).click();
      await page.waitForSelector("text=Root family");
    },
  },
  {
    name: "search",
    url: `/Genesis/1?texts=bhsa,sp,jps1917&search=1&sc=bhsa&q=${encodeURIComponent("word lex=HLK[ vt=infc")}`,
    ready: "text=Search in",
    act: async (page) => {
      await runSearch(page);
    },
  },
  {
    name: "search-terms",
    url: "/Genesis/12?texts=bhsa,sp",
    ready: ".plain-word",
    act: async (page) => {
      // לֶךְ in Genesis 12:1, an imperative of HLK[: its stem counts too
      await page.locator(".plain-word").nth(5).click();
      await page.getByText("Add as search term").click();
      await page.getByRole("checkbox", { name: /qal/ }).check();
      await page.mouse.move(1270, 750);
    },
  },
  {
    name: "statistics",
    url: `/Genesis/1?texts=bhsa,sp&search=1&sc=bhsa&q=${encodeURIComponent("word lex=HLK[ vt*")}`,
    ready: "text=Search in",
    act: async (page) => {
      await runSearch(page);
      await page.getByRole("button", { name: "Statistics", exact: true }).click();
      await page.waitForSelector("text=Values of");
      await page.waitForLoadState("networkidle");
    },
  },
  {
    name: "concordance",
    url: `/concordance/bhsa?q=${encodeURIComponent("word lex=HLK[ vt=infc")}&sort=L1.text`,
    ready: "text=Concordance",
    act: async (page) => {
      await runSearch(page);
      await page.waitForSelector("text=per 10,000 words");
    },
  },
  {
    name: "collocations",
    url: `/concordance/bhsa?q=${encodeURIComponent("word lex=HLK[ vt=infc")}&tab=collocations`,
    ready: "text=Concordance",
    act: async (page) => {
      await runSearch(page);
      await page.waitForSelector("text=logDice");
      await page.waitForLoadState("networkidle");
    },
  },
  {
    name: "library",
    url: "/Genesis/1?texts=bhsa,sp,dss",
    ready: "text=4Q2",
    act: async (page) => {
      await page.getByRole("button", { name: "Texts", exact: true }).click();
      await page.getByText("Add a text…").click();
      await page.waitForSelector("text=Dictionaries");
      await page.waitForLoadState("networkidle");
    },
    size: { width: 1280, height: 1100 },
  },
  {
    name: "workspaces",
    url: "/read/bhsa/Genesis/1?v=3&panel=notes",
    ready: "text=Running text",
    act: async (page) => {
      await page.getByRole("button", { name: "Workspaces", exact: true }).click();
      await page.getByPlaceholder("Name this workspace").fill("Genesis 1, with notes");
      await page.getByRole("button", { name: "Save", exact: true }).click();
      await page.waitForSelector("text=Export to a file");
    },
    clip: { x: 640, y: 0, width: 640, height: 420 },
  },
  {
    // needs the browser's secret, from ~/.cfabric/running.json of its home folder
    name: "proposal",
    url: "/Genesis/1?texts=bhsa,sp&search=1&sc=bhsa",
    prepare: (base) =>
      fetch(`${base}/api/proposals`, {
        method: "POST",
        headers: { "content-type": "application/json", "x-cfabric-secret": process.env.CFABRIC_SECRET ?? "" },
        body: JSON.stringify({
          corpus: "bhsa",
          template: "clause\n  p1:phrase function=Subj\n  p2:phrase function=Pred\np1 < p2",
          request: "clauses where the subject comes before the verb",
          explanation:
            "Clauses with a subject phrase and a predicate phrase, the subject first. The predicate phrase holds the verb (or, in a nominal clause, the predicate).",
          model: "Claude",
        }),
      }).then((r) => {
        if (!r.ok) throw new Error(`proposing: ${r.status} (set CFABRIC_SECRET)`);
      }),
    ready: "text=Proposed by Claude",
    // so that it doesn't show in the other shots
    after: (page) => page.getByRole("button", { name: "Dismiss", exact: true }).click(),
  },
];

/** Clicks *Search* in the search form (not the top bar's) and waits for the results. */
async function runSearch(page) {
  await page.getByRole("button", { name: "Search", exact: true }).last().click();
  await page.waitForSelector("text=/results in /", { timeout: 60_000 });
}

async function take(browser, base, shot) {
  const context = await browser.newContext({ viewport: shot.size ?? SIZE, deviceScaleFactor: 1 });
  await context.addInitScript((theme) => {
    try {
      localStorage.setItem("cfabric-browser/theme", theme);
    } catch {}
  }, shot.theme ?? "light");
  if (shot.prepare) await shot.prepare(base);
  const page = await context.newPage();
  await page.goto(base + shot.url);
  if (shot.ready) await page.waitForSelector(shot.ready, { timeout: 30_000 });
  await page.waitForLoadState("networkidle");
  if (shot.act) await shot.act(page);
  await page.waitForTimeout(400);
  const file = join(OUT, `${shot.name}.png`);
  await page.screenshot({ path: file, clip: shot.clip });
  if (shot.after) await shot.after(page);
  await context.close();
  console.log(`screenshot: ${file}`);
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  const [base = "http://127.0.0.1:8799", ...names] = process.argv.slice(2);
  mkdirSync(OUT, { recursive: true });
  const browser = await chromium.launch({ channel: "chrome" });
  let failed = 0;
  try {
    for (const shot of SHOTS) {
      if (names.length && !names.includes(shot.name)) continue;
      try {
        await take(browser, base, shot);
      } catch (e) {
        failed += 1;
        console.error(`screenshot ${shot.name} failed: ${e.message.split("\n").slice(0, 8).join("\n")}`);
      }
    }
  } finally {
    await browser.close();
  }
  process.exitCode = failed ? 1 : 0;
}
