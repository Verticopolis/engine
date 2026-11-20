import js from "@eslint/js";
import tseslint from "typescript-eslint";
import prettier from "eslint-config-prettier";
import globals from "globals";

// The same rules as the game's and the components' configs (Verticopolis/web,
// Verticopolis/components). Prettier owns layout; eslint-config-prettier turns
// ESLint's layout rules off. It covers every script in the repository: the
// packer (src/), the TypeScript layer, the build scripts and the docs site.
export default [
  {
    ignores: [
      "**/node_modules/",
      "**/dist/",
      "**/coverage/",
      "**/target/",
      "wasm/",
      "docs/engine-site/.astro/",
      "tooling/",
    ],
  },
  js.configs.recommended,
  ...tseslint.configs.recommended,
  prettier,
  {
    languageOptions: {
      ecmaVersion: 2022,
      sourceType: "module",
      globals: { ...globals.node },
    },
    rules: {
      "@typescript-eslint/no-explicit-any": "warn",
      "@typescript-eslint/no-unused-vars": ["error", { argsIgnorePattern: "^_", varsIgnorePattern: "^_" }],
      "prefer-const": "error",
      "no-var": "error",
      "no-nested-ternary": "error",
    },
  },
  {
    // The docs site's menu bar runs in the browser, on both kinds of page.
    files: ["docs/engine-site/menu.mjs", "docs/engine-site/rustdoc/*.mjs"],
    languageOptions: {
      globals: { ...globals.browser },
    },
  },
  {
    files: ["**/*.test.ts"],
    rules: {
      "@typescript-eslint/no-explicit-any": "off",
    },
  },
];
