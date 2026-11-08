/** Searchable model list (T40). Selection happens ONLY on explicit click
 * (PRD F04 — nothing is ever auto-selected, not on fetch, not on filter). */
import { useState } from "react";
import type { ModelEntry } from "../lib/ipc";
import { focusRing } from "../lib/focus";

export interface ModelListProps {
  models: ModelEntry[];
  selectedId: string | null;
  onSelect: (id: string) => void;
}

export function ModelList({ models, selectedId, onSelect }: ModelListProps) {
  const [query, setQuery] = useState("");

  const q = query.trim().toLowerCase();
  const filtered = q
    ? models.filter(
        (m) =>
          m.id.toLowerCase().includes(q) || (m.displayName?.toLowerCase().includes(q) ?? false),
      )
    : models;

  return (
    <div className="flex flex-col gap-2">
      <label htmlFor="model-search" className="sr-only">
        Filter models
      </label>
      <input
        id="model-search"
        type="search"
        value={query}
        onChange={(e) => setQuery(e.target.value)}
        placeholder="Filter by id or name…"
        className={`w-full rounded-md border border-zinc-300 bg-white px-3 py-2 text-sm text-zinc-700 placeholder-zinc-400 dark:border-zinc-600 dark:bg-zinc-800 dark:text-zinc-200 ${focusRing}`}
        aria-describedby="model-search-hint"
      />
      <p id="model-search-hint" className="text-xs text-zinc-500 dark:text-zinc-400">
        {filtered.length} of {models.length} models shown. Click a model to select it — nothing is
        selected automatically.
      </p>
      {filtered.length === 0 ? (
        <p className="text-sm text-zinc-500 dark:text-zinc-400" role="status">
          No models match “{query}”.
        </p>
      ) : (
        <ul
          aria-label="Discovered models"
          className="max-h-48 overflow-y-auto rounded-md border border-zinc-200 dark:border-zinc-700"
        >
          {filtered.map((m) => {
            const isSelected = m.id === selectedId;
            return (
              <li key={m.id}>
                <button
                  type="button"
                  role="option"
                  aria-selected={isSelected}
                  onClick={() => onSelect(m.id)}
                  className={`flex w-full items-center justify-between px-3 py-2 text-left text-sm ${focusRing} ${
                    isSelected
                      ? "bg-blue-50 text-blue-900 dark:bg-blue-950 dark:text-blue-200"
                      : "text-zinc-700 hover:bg-zinc-50 dark:text-zinc-200 dark:hover:bg-zinc-800"
                  }`}
                >
                  <span className="font-mono">{m.id}</span>
                  <span className="flex items-center gap-2 text-xs text-zinc-500 dark:text-zinc-400">
                    {m.displayName !== null && <span>{m.displayName}</span>}
                    <span className="rounded-full border border-zinc-300 px-1.5 py-0.5 dark:border-zinc-600">
                      capabilities unknown
                    </span>
                  </span>
                </button>
              </li>
            );
          })}
        </ul>
      )}
    </div>
  );
}
