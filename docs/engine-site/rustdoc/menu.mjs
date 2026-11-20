// The menu bar's script on the Rust API reference's pages (menu.mjs says what
// the bar holds). scripts/rustdoc-chrome.mjs bundles this, with the
// components' MenuBar, into one inline script after rustdoc's content.
// Options follows rustdoc's own theme settings, which the docs pages' menu
// writes too, so a look picked on either kind of page holds on both.
import { MenuBar } from "@verticopolis/components/windows/menuBar";
import { deviceTheme, mountMenu, storeTheme } from "../menu.mjs";

/** rustdoc's look as the menu names it: its theme, or the device's. */
function rustdocTheme() {
  try {
    if (localStorage.getItem("rustdoc-use-system-theme") !== "false") return "auto";
    return localStorage.getItem("rustdoc-theme") === "light" ? "light" : "dark";
  } catch {
    return "auto";
  }
}

/** Draw a look: rustdoc's light and dark themes are its built-in ones, which
 *  it draws from the root's data-theme alone. In High Contrast the head's
 *  script turns light back to dark (menu.mjs schemeBootScript). */
function showTheme(theme) {
  document.documentElement.setAttribute("data-theme", theme === "auto" ? deviceTheme() : theme);
}

mountMenu(
  MenuBar,
  {
    theme: rustdocTheme,
    setTheme: (theme) => {
      storeTheme(theme);
      showTheme(theme);
    },
    openSearch: () => window.searchState?.focus(),
    // Leaving High Contrast goes back to the look rustdoc keeps.
    afterScheme: () => showTheme(rustdocTheme()),
  },
  { find: "S" },
);
