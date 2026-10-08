#!/usr/bin/env node
/**
 * Hyper Engine website checks (DoD 8: "checks").
 * Zero-dependency Node validator:
 *   - every site/**\/*.html has lang="en", <title>, exactly one meta
 *     description and one <h1>, primary nav + legal footer nav with labels,
 *     and (except 404.html) a skip link
 *   - every local href/src resolves to a real file under site/
 *   - all pages are reachable from the home page nav in one hop
 * Usage:  node site/checks/sitecheck.mjs   (run from repo root)
 * Exits 1 on any failure; prints a report.
 */
import { readFileSync, existsSync, readdirSync, statSync } from "node:fs";
import { join, dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const site = resolve(dirname(fileURLToPath(import.meta.url)), "..", "..", "site");

function collectFiles(dir, out = []) {
  for (const entry of readdirSync(dir)) {
    const full = join(dir, entry);
    if (statSync(full).isDirectory()) collectFiles(full, out);
    else if (entry.endsWith(".html")) out.push(full);
  }
  return out;
}

const pages = collectFiles(site).map((f) => f.slice(site.length + 1));
if (pages.length === 0) {
  console.error("FAIL: no HTML pages found under site/");
  process.exit(1);
}

const failures = [];
const report = (ok, msg) => {
  console.log(`${ok ? "ok " : "FAIL"}  ${msg}`);
  if (!ok) failures.push(msg);
};

for (const rel of pages.sort()) {
  const file = join(site, rel);
  const html = readFileSync(file, "utf8");
  const base = dirname(file);

  report(/^<html lang="en">/m.test(html), `${rel}: lang="en"`);
  report(/\n<title>[^<]+<\/title>\n/.test(html), `${rel}: <title> on its own line`);
  const metas = [...html.matchAll(/<meta name="description" content="[^"]*"/g)];
  report(metas.length === 1, `${rel}: exactly one meta description (found ${metas.length})`);
  const h1count = (html.match(/<h1>/g) || []).length;
  report(h1count === 1, `${rel}: exactly one <h1> (found ${h1count})`);
  report(/aria-label="Primary"/.test(html), `${rel}: primary nav labelled`);
  report(/aria-label="Legal footer"/.test(html), `${rel}: legal footer nav labelled`);
  report(rel === "404.html" || /class="skip-link"/.test(html), `${rel}: skip link present`);

  const refs = [...html.matchAll(/(?:href|src)="([^"#][^"]*)"/g)]
    .map((m) => m[1])
    .filter((u) => !/^(https?:|mailto:)/.test(u));

  for (const href of refs) {
    const target = resolve(base, href.replace(/\/$/, ""));
    if (!target.startsWith(site)) {
      report(false, `${rel}: link escapes site/ (${href})`);
      continue;
    }
    report(existsSync(target), `${rel}: asset exists ${target.slice(site.length + 1)}`);
  }
}

const home = readFileSync(join(site, "index.html"), "utf8");
const navLinks = new Set([...home.matchAll(/href="([^"]+\.html)"/g)].map((m) => m[1]));
for (const rel of pages) {
  if (!["index.html", "404.html"].includes(rel) && !navLinks.has(rel)) {
    report(false, `${rel}: not linked from home nav`);
  }
}

console.log("");
if (failures.length === 0) {
  console.log(`PASS: ${pages.length} pages checked, all structure and link checks green.`);
} else {
  console.error(`FAIL: ${pages.length} pages checked, ${failures.length} problem(s) found:`);
  for (const f of failures) console.error(`  - ${f}`);
  process.exit(1);
}