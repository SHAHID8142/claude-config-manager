import { ChevronRight } from "lucide-react";
import { FieldRow } from "./FieldRow";
import { roleChecks, ROLE_FIELDS, type RoleKey, type RoleState } from "../lib/roles";

export interface AdvancedSettingsProps {
  open: boolean;
  onToggle: () => void;
  roles: RoleState;
  onChange: (key: RoleKey, value: string) => void;
  /** Live effective values from `read_status` (null = no override set). */
  current: Partial<Record<RoleKey, string | null>>;
  /** Declared context window in tokens, as typed ("" = not managed). */
  contextWindow: string;
  onContextWindowChange: (value: string) => void;
  /** Currently effective `CLAUDE_CODE_MAX_CONTEXT_TOKENS`, if any. */
  currentContextWindow: string | null;
}

/**
 * Advanced panel (T45): per-role model overrides, collapsed by default.
 * Default-no-override is the honest baseline — with empty fields the app
 * changes nothing, and each row shows what is currently configured so the
 * panel can never imply "not set" when an override exists.
 */
export function AdvancedSettings({
  open,
  onToggle,
  roles,
  onChange,
  current,
  contextWindow,
  onContextWindowChange,
  currentContextWindow,
}: AdvancedSettingsProps) {
  const checks = roleChecks(roles);
  return (
    <div className="mt-2 border-t border-line pt-2">
      <button
        type="button"
        aria-expanded={open}
        aria-controls="advanced-panel"
        onClick={onToggle}
        className="flex w-full items-center gap-1.5 rounded py-1 text-left text-[12.5px] font-medium text-ink-muted hover:text-ink focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-accent"
      >
        <ChevronRight
          aria-hidden="true"
          className={`size-3.5 transition-transform ${open ? "rotate-90" : ""}`}
        />
        Advanced settings
        <span className="font-normal text-ink-subtle">· roles and context window</span>
      </button>
      {open && (
        <div
          id="advanced-panel"
          role="group"
          aria-label="Advanced model settings"
          className="mt-2 flex flex-col gap-2"
        >
          <p className="text-xs text-ink-subtle">
            Leave a field blank to keep Claude Code&apos;s default for that role. Overrides are
            written to <code>settings.json</code> env only for the roles you fill in.
          </p>
          <FieldRow
            label="Context window (tokens)"
            htmlFor="context-window-input"
            hint={
              currentContextWindow !== null
                ? `Effective now: ${currentContextWindow} tokens · blank + Save removes the override`
                : "Only needed for model IDs Claude Code doesn't recognize — it assumes 200k otherwise."
            }
          >
            <input
              id="context-window-input"
              className="w-full rounded-[6px] border border-control bg-surface px-2.5 h-9 font-mono text-[12.5px] text-ink outline-none focus:border-accent"
              inputMode="numeric"
              placeholder="e.g. 1000000"
              value={contextWindow}
              onChange={(e) => onContextWindowChange(e.target.value.replace(/[^0-9]/g, ""))}
              aria-describedby="context-window-help"
            />
          </FieldRow>
          <p id="context-window-help" className="text-xs text-ink-subtle">
            Writes <code>CLAUDE_CODE_MAX_CONTEXT_TOKENS</code>. Claude Code sizes every model it
            cannot match to a Claude ID at 200k and auto-compacts there, so a 1M model behind
            OpenRouter or a gateway is cut off early without this value.
          </p>
          {ROLE_FIELDS.map(({ key, label, env }) => {
            const check = checks.find((c) => c.key === key);
            const invalid = check?.kind === "invalid";
            const effective = current[key] ?? null;
            return (
              <div key={key}>
                <FieldRow
                  label={label}
                  htmlFor={`role-${key}-input`}
                  hint={`Optional · overrides ${env}`}
                >
                  <input
                    id={`role-${key}-input`}
                    type="text"
                    value={roles[key]}
                    onChange={(e) => onChange(key, e.target.value)}
                    placeholder="No override (Claude Code default)"
                    aria-invalid={invalid}
                    aria-describedby={invalid ? `role-${key}-error` : undefined}
                    className="w-full rounded-[6px] border border-control bg-surface px-2.5 h-9 text-[13px] text-ink-muted placeholder:text-ink-subtle"
                  />
                </FieldRow>
                <p className="text-xs text-ink-muted">
                  Current:{" "}
                  {effective === null ? (
                    <span>no override (Claude Code default)</span>
                  ) : (
                    <code className="break-all">{effective}</code>
                  )}
                </p>
                {invalid && check !== undefined && (
                  <p
                    id={`role-${key}-error`}
                    aria-live="polite"
                    className="text-xs font-medium text-danger"
                  >
                    {check.message}
                  </p>
                )}
              </div>
            );
          })}
        </div>
      )}
    </div>
  );
}
