// @ts-check
// The Claude Overview site: a home page and usage docs, served by GitHub Pages from the twin at
// markpitt.dev/claude-overview/. Stand-alone: it builds from this folder alone, and the base path
// is config, so the same build serves at /claude-overview/, at / locally, or on another domain.
//
//   SITE_BASE   the path it serves under (default /claude-overview/)
//   SITE_URL    the origin, for canonical links (default https://markpitt.dev)
import { existsSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { defineConfig } from "astro/config";

const base = process.env.SITE_BASE ?? "/claude-overview/";

// The markpitt.dev chrome kit. A release copies it into src/chrome/; in the monorepo it is read
// from the one shared copy at the root. Both resolve through the same alias, so a dev build and
// a release build read the same files.
const vendored = fileURLToPath(new URL("./src/chrome", import.meta.url));
const shared = fileURLToPath(new URL("../../site-chrome", import.meta.url));
const chrome = existsSync(vendored) ? vendored : shared;

export default defineConfig({
  site: process.env.SITE_URL ?? "https://markpitt.dev",
  base,
  trailingSlash: "always",
  output: "static",
  compressHTML: true,
  vite: {
    resolve: { alias: { "@chrome": chrome } },
    // The screenshots live in ../docs/screenshots and, in the monorepo, the kit in ../../site-chrome.
    server: { fs: { allow: [fileURLToPath(new URL("../..", import.meta.url))] } },
  },
});
