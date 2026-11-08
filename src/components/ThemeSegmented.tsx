import { useState } from "react";
import { SegmentedControl, type SegmentedOption } from "./SegmentedControl";
import { applyTheme, loadTheme, saveTheme, type ThemeMode } from "../lib/theme";

const MODES: ReadonlyArray<SegmentedOption<ThemeMode>> = [
  { value: "system", label: "System" },
  { value: "light", label: "Light" },
  { value: "dark", label: "Dark" },
];

interface ThemeSegmentedProps {
  /** Injected in tests so the choice can be asserted without touching storage. */
  themeStorage?: Pick<Storage, "getItem" | "setItem"> | null;
}

/**
 * Appearance, where macOS settings puts it: a small control at the foot of the
 * sidebar. "System" follows the OS live — the root class is flipped by the
 * media query listener in lib/theme.ts, not only on change.
 */
export function ThemeSegmented({ themeStorage }: ThemeSegmentedProps) {
  const [mode, setMode] = useState<ThemeMode>(() => loadTheme(themeStorage ?? undefined));

  function choose(next: ThemeMode) {
    setMode(next);
    saveTheme(next, themeStorage ?? undefined);
    applyTheme(next);
  }

  return (
    <div className="mt-auto flex flex-col gap-1.5 px-3">
      <span className="text-[11px] text-ink-subtle">Appearance</span>
      <SegmentedControl label="Appearance" options={MODES} value={mode} onChange={choose} />
    </div>
  );
}
