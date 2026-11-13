/** Searchable model list (T40, restyled V2-13). Selection happens ONLY on
 * explicit click (PRD F04 — nothing is ever auto-selected, not on fetch, not on
 * filter). Rows stay one line tall so a 60-model catalog is scannable, and the
 * provenance line is stated once instead of repeating a chip on every row. */
import { useMemo, useState } from "react";
import { Check, Search } from "lucide-react";
import type { ModelEntry } from "../lib/ipc";
import { focusRing } from "../lib/focus";

export interface ModelListProps {
  models: ModelEntry[];
  selectedId: string | null;
  onSelect: (id: string) => void;
}

export function ModelList({ models, selectedId, onSelect }: ModelListProps) {
  const [query, setQuery] = useState("");

  // Every entry in one catalog shares the same verified source (T42); we show
  // it as provenance and never infer capability data from it.
  const rawSource = models[0]?.source ?? "";
  const source = rawSource.trim() === "" ? "unknown" : rawSource;

  const filtered = useMemo(() => {
    const q = query.trim().toLowerCase();
    if (q === "") return models;
    return models.filter(
      (m) => m.id.toLowerCase().includes(q) || (m.displayName?.toLowerCase().includes(q) ?? false),
    );
  }, [models, query]);

  return (
    <div className="flex flex-col gap-2">
      <div className="relative">
        <Search
          aria-hidden="true"
          className="pointer-events-none absolute top-1/2 left-2.5 size-3.5 -translate-y-1/2 text-ink-subtle"
        />
        <label htmlFor="model-search" className="sr-only">
          Filter models
        </label>
        <input
          id="model-search"
          type="search"
          value={query}
          onChange={(e) => setQuery(e.target.value)}
          placeholder="Filter by id or name"
          className={`w-full rounded-md border border-control bg-surface py-1.5 pr-2.5 pl-8 text-[12.5px] text-ink placeholder:text-ink-subtle ${focusRing}`}
          aria-describedby="model-search-hint"
        />
      </div>
      <p id="model-search-hint" className="text-[11px] text-ink-subtle">
        {filtered.length} of {models.length} models · click to select, nothing is auto-selected
      </p>

      {filtered.length === 0 ? (
        <p className="py-3 text-[12.5px] text-ink-muted" role="status">
          No models match “{query}”.
        </p>
      ) : (
        <ul
          aria-label="Discovered models"
          className="max-h-56 overflow-y-auto rounded-md border border-control bg-surface"
        >
          {filtered.map((m) => {
            const isSelected = m.id === selectedId;
            return (
              <li key={m.id} className="border-b border-line last:border-b-0">
                <button
                  type="button"
                  role="option"
                  data-testid="model-row"
                  aria-selected={isSelected}
                  onClick={() => onSelect(m.id)}
                  className={`flex h-8 w-full items-center gap-2 px-2.5 text-left text-[12.5px] ${focusRing} ${
                    isSelected
                      ? "bg-accent-soft text-ink"
                      : "text-ink-muted hover:bg-surface-sunken hover:text-ink"
                  }`}
                >
                  <Check
                    aria-hidden="true"
                    className={`size-3.5 shrink-0 ${isSelected ? "text-accent" : "invisible"}`}
                  />
                  <span className="min-w-0 flex-1 truncate font-mono">{m.id}</span>
                  {m.displayName !== null && (
                    <span className="max-w-40 shrink-0 truncate text-[11px] text-ink-subtle">
                      {m.displayName}
                    </span>
                  )}
                </button>
              </li>
            );
          })}
        </ul>
      )}

      <p className="text-[11px] text-ink-subtle">
        Source: <span className="font-mono">{source}</span> · capability data unverified (unknown
        for every model)
      </p>
    </div>
  );
}
