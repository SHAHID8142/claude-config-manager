import { FieldRow } from "./components/FieldRow";
import { Section } from "./components/Section";

/**
 * Minimal single-window layout (T12).
 * Structural shell for the PRD main screen; individual controls become
 * functional in their own tasks (T18–T20 provider/key fields, T33 apply,
 * T37+ model selection, T45 advanced panel). Controls that are not wired
 * yet render disabled with honest "not configured" placeholders — never
 * fabricated status.
 */
export default function App() {
  return (
    <main className="mx-auto flex min-h-screen max-w-2xl flex-col gap-4 bg-zinc-50 p-6 dark:bg-zinc-950">
      <header className="flex items-center justify-between">
        <h1 className="text-lg font-semibold text-zinc-900 dark:text-zinc-100">
          Claude Config Manager
        </h1>
        <span
          aria-label="Connection mode"
          className="rounded-full border border-zinc-300 px-2 py-0.5 text-xs text-zinc-500 dark:border-zinc-600 dark:text-zinc-400"
        >
          mode: not configured
        </span>
      </header>

      <Section title="Active configuration">
        <p className="text-sm text-zinc-500 dark:text-zinc-400">
          No configuration loaded yet. Settings reading arrives with the configuration-safety
          milestone.
        </p>
      </Section>

      <Section title="Provider">
        <FieldRow
          label="Provider"
          htmlFor="provider-select"
          hint="Providers appear once verified presets ship."
        >
          <select
            id="provider-select"
            disabled
            className="w-full rounded-md border border-zinc-300 bg-white px-3 py-2 text-sm text-zinc-700 disabled:cursor-not-allowed disabled:opacity-50 dark:border-zinc-600 dark:bg-zinc-800 dark:text-zinc-200"
          >
            <option>No providers yet</option>
          </select>
        </FieldRow>
        <FieldRow
          label="Endpoint URL"
          htmlFor="endpoint-input"
          hint="Editable when a provider is selected."
        >
          <input
            id="endpoint-input"
            type="url"
            disabled
            placeholder="https://"
            className="w-full rounded-md border border-zinc-300 bg-white px-3 py-2 text-sm text-zinc-700 placeholder-zinc-400 disabled:cursor-not-allowed disabled:opacity-50 dark:border-zinc-600 dark:bg-zinc-800 dark:text-zinc-200"
          />
        </FieldRow>
        <FieldRow
          label="API key"
          htmlFor="api-key-input"
          hint="Stored in the macOS Keychain when saving."
        >
          <input
            id="api-key-input"
            type="password"
            disabled
            placeholder="••••••••••••"
            className="w-full rounded-md border border-zinc-300 bg-white px-3 py-2 text-sm text-zinc-700 disabled:cursor-not-allowed disabled:opacity-50 dark:border-zinc-600 dark:bg-zinc-800 dark:text-zinc-200"
          />
        </FieldRow>
      </Section>

      <Section title="Model">
        <FieldRow
          label="Model"
          htmlFor="model-input"
          hint="Model discovery and selection arrive with the model milestone; nothing is ever auto-selected."
        >
          <input
            id="model-input"
            type="text"
            disabled
            placeholder="provider-model-id"
            className="w-full rounded-md border border-zinc-300 bg-white px-3 py-2 text-sm text-zinc-700 placeholder-zinc-400 disabled:cursor-not-allowed disabled:opacity-50 dark:border-zinc-600 dark:bg-zinc-800 dark:text-zinc-200"
          />
        </FieldRow>
      </Section>

      <Section title="Actions">
        <div className="flex flex-wrap gap-2">
          <button
            type="button"
            disabled
            className="rounded-md border border-zinc-300 px-3 py-1.5 text-sm font-medium text-zinc-700 disabled:cursor-not-allowed disabled:opacity-50 dark:border-zinc-600 dark:text-zinc-200"
          >
            Test Connection
          </button>
          <button
            type="button"
            disabled
            className="rounded-md bg-blue-600 px-3 py-1.5 text-sm font-medium text-white disabled:cursor-not-allowed disabled:opacity-50"
          >
            Save &amp; Apply
          </button>
        </div>
      </Section>

      <details className="rounded-lg border border-zinc-200 bg-white p-4 dark:border-zinc-700 dark:bg-zinc-900">
        <summary className="cursor-pointer text-sm font-semibold text-zinc-700 dark:text-zinc-200">
          Advanced
        </summary>
        <p className="mt-2 text-sm text-zinc-500 dark:text-zinc-400">
          Advanced overrides arrive in a later task and default to OFF.
        </p>
      </details>
    </main>
  );
}
