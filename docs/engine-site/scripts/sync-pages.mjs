// Copies the repository files the docs site shows into the Starlight content
// folder, so the READMEs and guides stay the only source. Run before every
// build (`npm run build` does). Each page in pages.json becomes
// src/content/docs/<slug>.md with front matter, its own top heading dropped
// (Starlight prints the title), and its links rewritten: a link to another
// page of the site stays on the site, and any other repository link points at
// GitHub at DOCS_REF (the release tag on a release build, main otherwise). A
// mermaid block becomes a <pre class="mermaid"> that the page draws as a
// diagram in the browser (src/components/Mermaid.astro), as GitHub draws it.
// The game's logo, favicon and title-screen skyline are copied in the same way;
// its palette comes from @verticopolis/components (astro.config.mjs).
import { copyFileSync, mkdirSync, readFileSync, statSync, writeFileSync } from "node:fs";
import { dirname, join, posix, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const site = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const repo = resolve(site, "..", "..");
const ref = process.env.DOCS_REF || "main";
const GITHUB = "https://github.com/Verticopolis/engine";
const { pages, aliases } = JSON.parse(readFileSync(join(site, "pages.json"), "utf8"));

/** Repository path (a file, or a folder whose README is a page) to its site page. */
const pageFor = new Map();
for (const page of pages) {
  pageFor.set(page.source, page.slug);
  if (posix.basename(page.source) === "README.md") pageFor.set(posix.dirname(page.source), page.slug);
}

/** A file the package gets at pack time, as the file it is copied from. */
function unalias(repoPath) {
  return aliases[repoPath] ?? repoPath;
}

function siteLink(repoPath, anchor) {
  const slug = pageFor.get(repoPath.replace(/\/$/, ""));
  return slug === undefined ? undefined : `/${slug}/${anchor}`;
}

function githubLink(repoPath, anchor) {
  let kind = "blob";
  try {
    if (statSync(join(repo, repoPath)).isDirectory()) kind = "tree";
  } catch {
    // A path that does not exist still gets a link; the review catches it.
  }
  return `${GITHUB}/${kind}/${ref}/${repoPath}${anchor}`;
}

/** Rewrites one link target found in `source` (a repository path). */
export function rewriteTarget(target, source) {
  const hash = target.indexOf("#");
  const path = hash === -1 ? target : target.slice(0, hash);
  const anchor = hash === -1 ? "" : target.slice(hash);
  if (path === "") return target;
  const ours = path.match(/^https:\/\/github\.com\/Verticopolis\/engine\/(?:blob|tree)\/main\/(.+)$/);
  if (ours) return siteLink(unalias(ours[1]), anchor) ?? githubLink(unalias(ours[1]), anchor);
  if (/^[a-z][a-z0-9+.-]*:/i.test(path) || path.startsWith("/")) return target;
  const repoPath = unalias(posix.normalize(posix.join(posix.dirname(source), path)));
  return siteLink(repoPath, anchor) ?? githubLink(repoPath, anchor);
}

function escapeHtml(text) {
  return text.replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;");
}

/** The page body: links rewritten and mermaid blocks marked outside code
 *  fences, and the page's own top heading dropped. */
export function convert(markdown, source) {
  const out = [];
  let fence = null; // the fence that opened the current code block
  let mermaid = null; // the lines of the mermaid block being read
  for (const line of markdown.split("\n")) {
    const marker = line.match(/^\s*(```+|~~~+)\s*(\S*)/);
    if (mermaid) {
      if (marker && marker[1] === fence) {
        out.push(`<pre class="mermaid">${escapeHtml(mermaid.join("\n"))}</pre>`);
        mermaid = null;
        fence = null;
      } else mermaid.push(line);
      continue;
    }
    if (fence) {
      if (marker && marker[1] === fence) fence = null;
      out.push(line);
      continue;
    }
    if (marker) {
      fence = marker[1];
      if (marker[2] === "mermaid") mermaid = [];
      else out.push(line);
      continue;
    }
    out.push(
      line
        .replace(/\]\(([^)\s]+)\)/g, (_, target) => `](${rewriteTarget(target, source)})`)
        .replace(/^(\s*\[[^\]]+\]:\s*)(\S+)/, (_, label, target) => label + rewriteTarget(target, source)),
    );
  }
  // The top heading, even under a note quoted above it (a copied record
  // opens with one), since Starlight prints the title.
  const top = out.findIndex((line) => line.trim() !== "" && !line.startsWith(">"));
  if (top !== -1 && out[top].startsWith("# ")) out.splice(top, 1);
  return out.join("\n").replace(/^\n+/, "");
}

function frontMatter(page) {
  const quote = (text) => JSON.stringify(text);
  return `---\ntitle: ${quote(page.title)}\ndescription: ${quote(page.description)}\n---\n\n`;
}

function main() {
  const docs = join(site, "src/content/docs");
  mkdirSync(docs, { recursive: true });
  for (const page of pages) {
    const markdown = readFileSync(join(repo, page.source), "utf8");
    writeFileSync(join(docs, `${page.slug}.md`), frontMatter(page) + convert(markdown, page.source));
  }
  mkdirSync(join(site, "src/assets"), { recursive: true });
  mkdirSync(join(site, "public"), { recursive: true });
  copyFileSync(join(site, "brand/icon-1024x1024.png"), join(site, "src/assets/logo.png"));
  copyFileSync(join(site, "brand/favicon.png"), join(site, "public/favicon.png"));
  console.log(`synced ${pages.length} pages at ${ref}`);
}

if (process.argv[1] === fileURLToPath(import.meta.url)) main();
