// Writes the docs site's header and side menu for the Rust API reference.
// cargo doc writes its own pages under /rust/, so the workflow passes these
// three files to rustdoc (engine-docs.yml):
//
//   head.html    --html-in-header: the game's palette (retro-tokens.css from
//                @verticopolis/components, so run `npm ci` here first), the
//                shared title bar's and the menu bar's styles and the
//                appearance schemes' (styles/title-bar.css, menu-bar.css and
//                starlight-schemes.css, also from the components),
//                rustdoc/theme.css, and the script that puts on the scheme
//                picked in the game (menu.mjs)
//   before.html  --html-before-content: the place the game's menu bar is
//                drawn, the navy title bar and the site menu
//   after.html   --html-after-content: moves the site menu to the top of
//                rustdoc's sidebar, above the crate's own items, and on a
//                phone folds the title bar into rustdoc's own top bar, so
//                the page has one bar and one menu button; then loads
//                vc-menu.js, which draws the menu bar
//   vc-menu.js   the menu bar's script (rustdoc/menu.mjs and the components'
//                MenuBar, bundled into one file), which the workflow copies
//                to /rust/ so every page shares one cached copy
//
// The title and the menu come from site.mjs and pages.json, the same lists
// Starlight draws from, so the two stay one menu. The title bar's markup is
// title-bar.mjs, the one the docs site and the help site draw, and the menu
// bar's model is menu.mjs, the one the docs site draws.
//
// Usage: node scripts/rustdoc-chrome.mjs <output folder>
import { mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import postcss from "postcss";
import { build } from "rolldown";
import { MENU_HOST, schemeBootScript } from "../menu.mjs";
import { GITHUB, RUST_REFERENCE, SHORT_TITLE, sidebar, siteTitle } from "../site.mjs";
import { barLinks, escapeHtml as escape, siteName } from "../title-bar.mjs";

/** Where the Rust API reference's pages load the menu bar's script from. */
const MENU_SCRIPT = "/rust/vc-menu.js";

const site = resolve(dirname(fileURLToPath(import.meta.url)), "..");

/** One menu entry as a link; a slug takes its title and path from pages.json. */
function link(entry, titles) {
  const label = entry.label ?? titles.get(entry.slug);
  if (label === undefined) throw new Error(`site.mjs names a page pages.json does not list: ${entry.slug}`);
  const href = entry.link ?? `/${entry.slug}/`;
  const current = href === RUST_REFERENCE ? ' aria-current="page"' : "";
  return `<li><a href="${escape(href)}"${current}>${escape(label)}</a></li>`;
}

/** The site menu, drawn the way Starlight draws it: top links, then labeled groups. */
export function siteMenu(entries, titles) {
  const items = entries.map((entry) =>
    entry.items
      ? `<li><details open><summary>${escape(entry.label)}</summary><ul>${entry.items
          .map((item) => link(item, titles))
          .join("")}</ul></details></li>`
      : link(entry, titles).replace("<li>", '<li class="vc-top">'),
  );
  // A phone's title bar has no room for GitHub, so the menu ends with it
  // there, as Starlight's phone menu does.
  items.push(`<li class="vc-top vc-menu-github"><a href="${GITHUB}">GitHub</a></li>`);
  return `<nav class="vc-site-menu" aria-label="Docs"><ul>${items.join("")}</ul></nav>`;
}

/** A components stylesheet, inlined for rustdoc's pages. The components put
 *  every rule in a cascade layer under `vc`, and a layered rule loses to any
 *  unlayered one. rustdoc's own stylesheets are unlayered, so here the rules
 *  come out of their layers (and the @import lines go, since the head inlines
 *  each sheet it needs in order); they then win over rustdoc's by
 *  specificity and order, as the docs site's own copies did. */
function components(sheet) {
  const path = fileURLToPath(import.meta.resolve(`@verticopolis/components/styles/${sheet}`));
  const root = postcss.parse(readFileSync(path, "utf8"));
  root.walkAtRules("import", (rule) => rule.remove());
  // One pass unwraps one level: a walk skips the rules a replaced block moves
  // up, so a layer nested in another layer needs the next pass.
  let layered;
  do {
    layered = false;
    root.walkAtRules("layer", (rule) => {
      layered = true;
      if (rule.nodes) rule.replaceWith(rule.nodes);
      else rule.remove();
    });
  } while (layered);
  return root.toString();
}

/** The menu bar's script for rustdoc's pages: rustdoc/menu.mjs and what it
 *  imports, in one file that runs as the page loads. The bar's styles come
 *  in the head, so the CSS the components' MenuBar imports is left out. */
async function menuScript() {
  const result = await build({
    input: join(site, "rustdoc/menu.mjs"),
    cwd: site,
    write: false,
    output: { format: "iife", minify: true },
    plugins: [
      {
        name: "styles-in-the-head",
        resolveId: (id) => (id.endsWith(".css") ? "\0css" : null),
        load: (id) => (id === "\0css" ? "export {};" : null),
      },
    ],
  });
  return result.output[0].code;
}

export async function chrome(ref) {
  const { pages } = JSON.parse(readFileSync(join(site, "pages.json"), "utf8"));
  const titles = new Map(pages.map((page) => [page.slug, page.title]));
  const style = [
    components("retro-tokens.css"),
    components("title-bar.css"),
    components("menu-bar.css"),
    components("starlight-schemes.css"),
    readFileSync(join(site, "rustdoc/theme.css"), "utf8"),
  ].join("\n");
  const head = `<style>\n${style}</style>\n<script>${schemeBootScript({ rustdoc: true })}</script>\n`;
  const title = `<a class="vc-bar-title" href="/"><img src="/favicon.png" alt="" width="40" height="40">${siteName(siteTitle(ref), SHORT_TITLE)}</a>`;
  const before =
    `<div class="vc-menubar-host" id="${MENU_HOST}"></div>\n` +
    `<header class="vc-header"><div class="vc-bar">${title}<div class="vc-bar-links">${barLinks()}</div></div></header>\n` +
    `${siteMenu(sidebar, titles)}\n`;
  // The site menu moves into rustdoc's sidebar, which a phone opens with
  // rustdoc's menu button. A phone shows rustdoc's top bar (its menu button,
  // settings and help) in place of the title bar, so the title and the links
  // join it there; rustdoc/theme.css hides the title bar and rustdoc's own
  // heading in that bar once the class is set.
  const after =
    "<script>(function(){" +
    'var d=document.documentElement,m=document.querySelector(".vc-site-menu"),s=document.querySelector("nav.sidebar");' +
    'if(m&&s){s.insertBefore(m,s.firstChild);d.classList.add("vc-menu-in-sidebar");}' +
    'var t=document.querySelector("rustdoc-topbar"),b=document.querySelector(".vc-header .vc-bar");' +
    'if(t&&b){for(var i=0;i<b.children.length;i++)t.appendChild(b.children[i].cloneNode(true));d.classList.add("vc-one-bar");}' +
    "})();</script>\n" +
    `<script src="${MENU_SCRIPT}"></script>\n`;
  return { head, before, after, menu: await menuScript() };
}

if (process.argv[1] === fileURLToPath(import.meta.url)) {
  const out = process.argv[2];
  if (!out) throw new Error("usage: node scripts/rustdoc-chrome.mjs <output folder>");
  mkdirSync(out, { recursive: true });
  const files = await chrome(process.env.DOCS_REF ?? "");
  writeFileSync(join(out, "head.html"), files.head);
  writeFileSync(join(out, "before.html"), files.before);
  writeFileSync(join(out, "after.html"), files.after);
  writeFileSync(join(out, "vc-menu.js"), files.menu);
  console.log(`wrote the Rust API reference's header and menu to ${out}`);
}
