// The game's Win95 menu bar across the top of every docs page, as the help
// site draws it (owner, 2026-11-20): File (the game, print), Edit (find, copy
// this page's link), Options (light, dark or the device's look, and the
// scheme) and Help (what's new, report a problem, sponsor). The Starlight
// pages draw it with src/components/MenuBar.astro and the Rust API
// reference's pages with rustdoc/menu.mjs, both from the model here. The
// bar's markup, look and behavior (opening, pointing, the keys) are the
// game's, from @verticopolis/components (windows/menuBar draws the model,
// styles/menu-bar.css); this file only says what each item is and does.
//
// The page also wears one of the game's appearance schemes (Standard,
// Skyline or High Contrast): the one picked in Options here, or else the one
// picked in the game (Options > Appearance…), which the game keeps in its
// vc_scheme cookie for all of verticopolis.com, as the help site reads it.
import { GAME, GITHUB } from "./title-bar.mjs";

/** Report a problem with the engine or these docs. */
export const ISSUES = `${GITHUB}/issues/new/choose`;

export const SPONSOR_URL = "https://github.com/sponsors/maniator";

/** What's New: the engine changelog page. */
export const CHANGELOG = "/changelog/";

/** The game's appearance schemes, by the names its cookie and html[data-scheme] use. */
export const SCHEMES = [
  { value: "standard", label: "Standard" },
  { value: "skyline", label: "Skyline" },
  { value: "high_contrast", label: "High Contrast" },
];

/** Where a scheme picked on these pages is kept (both kinds of page share it). */
export const SCHEME_KEY = "vc-docs-scheme";

/** The looks the Options menu offers. */
const THEMES = [
  { value: "light", label: "Light" },
  { value: "dark", label: "Dark" },
  { value: "auto", label: "Match My Device" },
];

/** Where each kind of page draws the bar; `mountMenu` renders into it. */
export const MENU_HOST = "docs-menubar-host";

/**
 * The look picked here, as Starlight keeps it (`starlight-theme`: light, dark,
 * or empty for the device's). Both kinds of page read and write it, so a pick
 * on one carries to the other.
 */
export function storedTheme() {
  try {
    const value = localStorage.getItem("starlight-theme");
    return value === "light" || value === "dark" ? value : "auto";
  } catch {
    return "auto";
  }
}

/**
 * Keep a pick for both kinds of page: Starlight's key, and rustdoc's own
 * settings (its theme and whether it follows the device), so the Rust API
 * reference opens in the same look and its own settings page agrees.
 */
export function storeTheme(theme) {
  try {
    localStorage.setItem("starlight-theme", theme === "auto" ? "" : theme);
    localStorage.setItem("rustdoc-use-system-theme", String(theme === "auto"));
    if (theme !== "auto") localStorage.setItem("rustdoc-theme", theme);
  } catch {
    // Without storage the pick still holds on this page.
  }
}

/** The look the device asks for. */
export function deviceTheme() {
  return matchMedia("(prefers-color-scheme: light)").matches ? "light" : "dark";
}

/**
 * The bar as the components' MenuBar draws it, with the look and scheme in
 * use checked. `deps` says how this kind of page reads and sets its look and
 * opens its search; `find` is the search's shortcut on these pages.
 */
export function menuModel({ theme, setTheme, openSearch, afterScheme = () => {} }, { find = "Ctrl+K" } = {}) {
  const look = theme();
  const scheme = currentScheme();
  const radio = (label, checked, onPick) => ({ label, checked, radio: true, onPick });
  const away = { target: "_blank", rel: "noopener noreferrer" };
  return {
    id: "docs-menubar",
    label: "Docs menu",
    className: "starlight-menubar print:hidden",
    menus: [
      {
        id: "dm-file",
        label: "File",
        mnemonic: 0,
        items: [
          { label: "Play Verticopolis", href: GAME },
          { kind: "separator" },
          { label: "Print…", shortcut: "Ctrl+P", onPick: () => window.print() },
        ],
      },
      {
        id: "dm-edit",
        label: "Edit",
        mnemonic: 0,
        items: [
          { label: "Find…", shortcut: find, onPick: () => openSearch() },
          {
            label: "Copy Link to This Page",
            onPick: () => void navigator.clipboard?.writeText(location.origin + location.pathname).catch(() => {}),
          },
        ],
      },
      {
        id: "dm-options",
        label: "Options",
        mnemonic: 0,
        items: [
          ...THEMES.map(({ value, label }) => radio(label, value === look, () => setTheme(value))),
          { kind: "separator" },
          ...SCHEMES.map(({ value, label }) =>
            radio(label, value === scheme, () => {
              setScheme(value);
              afterScheme();
            }),
          ),
        ],
      },
      {
        id: "dm-help",
        label: "Help",
        mnemonic: 0,
        items: [
          { label: "What's New", href: CHANGELOG },
          { label: "Report a Problem…", href: ISSUES, ...away },
          { kind: "separator" },
          { label: "Sponsor Verticopolis", href: SPONSOR_URL, ...away },
        ],
      },
    ],
  };
}

/**
 * Draw the bar into the page's MENU_HOST with the components' `MenuBar`
 * class. Opening a menu draws it again, so Options checks the look and scheme
 * in use.
 */
export function mountMenu(MenuBar, deps, options) {
  const root = document.getElementById(MENU_HOST);
  if (!root) return null;
  const bar = new MenuBar(root, menuModel(deps, options), {
    onOpen: () => bar.render(menuModel(deps, options)),
  });
  return bar;
}

/** The scheme the page wears now. */
export function currentScheme() {
  return document.documentElement.getAttribute("data-scheme") ?? "standard";
}

/** Wear a scheme on this page and keep it for the next. */
export function setScheme(scheme) {
  if (scheme === "standard") document.documentElement.removeAttribute("data-scheme");
  else document.documentElement.setAttribute("data-scheme", scheme);
  try {
    localStorage.setItem(SCHEME_KEY, scheme);
  } catch {
    // Without storage the pick lasts for this page.
  }
}

/** A value as a JavaScript literal that is safe inside an inline <script>:
 *  the characters that could close the tag or end a line are escaped. */
function scriptLiteral(value) {
  return JSON.stringify(value).replace(
    /[<>/\u2028\u2029]/g,
    (c) => `\\u${c.charCodeAt(0).toString(16).padStart(4, "0")}`,
  );
}

/**
 * An inline script for the page's head, run before it paints: the scheme
 * picked here (SCHEME_KEY), or else the one picked in the game, from its
 * vc_scheme cookie (Domain=verticopolis.com), on the page's root as
 * data-scheme, which the components' scheme rules read. Standard, nothing
 * picked or an unknown value leaves the page as it is.
 *
 * High Contrast is white on black whatever the page's own light or dark
 * setting, as on the help site. The Rust API reference's pages
 * (`rustdoc: true`) get there from rustdoc's dark theme, whose code colors
 * read on black, so while High Contrast is on the script holds them on it.
 */
export function schemeBootScript({ rustdoc = false } = {}) {
  const known = scriptLiteral(SCHEMES.map(({ value }) => value));
  const dark = rustdoc
    ? 'var f=function(){if(d.getAttribute("data-scheme")==="high_contrast"&&d.getAttribute("data-theme")==="light")d.setAttribute("data-theme","dark")};f();new MutationObserver(f).observe(d,{attributes:true,attributeFilter:["data-theme","data-scheme"]});'
    : "";
  return `(function(){try{var d=document.documentElement,s=null;try{s=localStorage.getItem(${scriptLiteral(SCHEME_KEY)})}catch(e){}if(${known}.indexOf(s)<0){var m=/(?:^|; *)vc_scheme=([a-z_]+)/.exec(document.cookie);s=m?m[1]:null}if(s!=="standard"&&${known}.indexOf(s)>=0)d.setAttribute("data-scheme",s);${dark}}catch(e){}})();`;
}
