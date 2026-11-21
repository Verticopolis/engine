// The engine developer docs at docs.verticopolis.com. The pages come from the
// repository's own READMEs and guides (pages.json, scripts/sync-pages.mjs);
// the look is the game's, from @verticopolis/components (its Starlight theme,
// styles/starlight.css, over the game's palette). .github/workflows/engine-docs.yml
// builds it with the crate's Rust API reference under /rust/.
import { defineConfig } from "astro/config";
import starlight from "@astrojs/starlight";
import { GITHUB, sidebar, siteTitle } from "./site.mjs";
import { schemeBootScript } from "./menu.mjs";

// DOCS_REF names the ref the pages link to; the workflow sets it to main.
const title = siteTitle(process.env.DOCS_REF);

// DOCS_BASE puts a pull request's preview under /pr-preview/pr-<number>/
// (docs-preview.yml); the live site sits at the root.
export default defineConfig({
  site: "https://docs.verticopolis.com",
  base: process.env.DOCS_BASE || "/",
  integrations: [
    starlight({
      title,
      description:
        "Developer docs for the Verticopolis simulation engine: the JavaScript package, the Rust crate, the host API and the conformance suite.",
      logo: { src: "./src/assets/logo.png", alt: "Verticopolis" },
      favicon: "/favicon.png",
      social: [{ icon: "github", label: "GitHub", href: GITHUB }],
      components: {
        Header: "./src/components/Header.astro",
        Hero: "./src/components/Hero.astro",
      },
      // Starlight's CSS sits in the `starlight` layer and the components' in
      // `vc`; this orders them before any stylesheet loads, so the game look
      // wins (the components README, "Styles").
      // The scheme picked in the game goes on the page before it paints
      // (menu.mjs).
      head: [
        { tag: "style", content: "@layer starlight, vc;" },
        { tag: "script", content: schemeBootScript() },
      ],
      // The game's look, then its menu bar over the title bar (MenuBar.astro).
      customCss: [
        "@verticopolis/components/styles/starlight.css",
        "@verticopolis/components/styles/starlight-menu-bar.css",
      ],
      sidebar,
    }),
  ],
});
