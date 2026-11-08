/**
 * V2-16: the token pairs the UI actually puts on screen must pass WCAG 2.2
 * contrast in BOTH themes. Reading the real stylesheet (not a copy of the
 * values) means a future palette tweak cannot silently fail the check.
 */
import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import { describe, expect, it } from "vitest";

const css = readFileSync(resolve(__dirname, "theme.css"), "utf8");

function block(selector: string): Record<string, string> {
  const start = css.indexOf(selector);
  if (start === -1) throw new Error(`missing block ${selector}`);
  const body = css.slice(start, css.indexOf("}", start));
  const tokens: Record<string, string> = {};
  for (const m of body.matchAll(/(--[a-z-]+):\s*(#[0-9a-f]{6})/gi)) tokens[m[1]] = m[2];
  return tokens;
}

function luminance(hex: string): number {
  const [r, g, b] = [1, 3, 5].map((i) => parseInt(hex.slice(i, i + 2), 16) / 255);
  const f = (c: number) => (c <= 0.03928 ? c / 12.92 : ((c + 0.055) / 1.055) ** 2.4);
  return 0.2126 * f(r) + 0.7152 * f(g) + 0.0722 * f(b);
}

function contrast(a: string, b: string): number {
  const [hi, lo] = [luminance(a), luminance(b)].sort((x, y) => y - x);
  return (hi + 0.05) / (lo + 0.05);
}

const themes = {
  light: block(":root {"),
  dark: block(".dark {"),
};

describe("theme contrast (V2-16)", () => {
  for (const [name, t] of Object.entries(themes)) {
    it(`${name}: body ink is AAA and muted/subtle text is AA`, () => {
      const bg = t["--app-bg"];
      expect(contrast(t["--ink"], bg)).toBeGreaterThanOrEqual(7);
      expect(contrast(t["--ink-muted"], bg)).toBeGreaterThanOrEqual(4.5);
      expect(contrast(t["--ink-subtle"], bg)).toBeGreaterThanOrEqual(4.5);
      // Raised cards sit on the app background, so they must not be worse.
      expect(contrast(t["--ink"], t["--surface"])).toBeGreaterThanOrEqual(7);
      expect(contrast(t["--ink-muted"], t["--surface"])).toBeGreaterThanOrEqual(4.5);
    });

    it(`${name}: the accent button label stays readable`, () => {
      expect(contrast(t["--ink-on-accent"], t["--accent"])).toBeGreaterThanOrEqual(4.5);
      expect(contrast(t["--ink-on-accent"], t["--accent-hover"])).toBeGreaterThanOrEqual(4.5);
    });

    it(`${name}: state colours are distinguishable from the surface`, () => {
      for (const key of ["--ok", "--warn", "--danger"]) {
        expect(contrast(t[key], t["--app-bg"]), key).toBeGreaterThanOrEqual(3);
      }
    });
  }
});
