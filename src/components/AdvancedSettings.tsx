import { FieldRow } from "./FieldRow";
import { roleChecks, ROLE_FIELDS, type RoleKey, type RoleState } from "../lib/roles";

export interface AdvancedSettingsProps {
  open: boolean;
  onToggle: () => void;
  roles: RoleState;
  onChange: (key: RoleKey, value: string) => void;
  /** Live effective values from `read_status` (null = no override set). */
  current: Partial<Record<RoleKey, string | null>>;
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
}: AdvancedSettingsProps) {
  const checks = roleChecks(roles);
  return (
    <div className="mt-3 border-t border-zinc-200 pt-3 dark:border-zinc-700">
      <button
        type="button"
        aria-expanded={open}
        aria-controls="advanced-panel"
        onClick={onToggle}
        className="text-sm font-medium text-zinc-700 underline-offset-2 hover:underline dark:text-zinc-300"
      >
        Advanced settings (model roles) <span aria-hidden="true">{open ? "▾" : "▸"}</span>
      </button>
      {open && (
        <div
          id="advanced-panel"
          role="group"
          aria-label="Advanced model role overrides"
          className="mt-2 flex flex-col gap-2"
        >
          <p className="text-xs text-zinc-500 dark:text-zinc-400">
            Leave a field blank to keep Claude Code&apos;s default for that role. Overrides are
            written to <code>settings.json</code> env only for the roles you fill in.
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
                    className="w-full rounded-md border border-zinc-300 bg-white px-3 py-2 text-sm text-zinc-700 placeholder-zinc-400 dark:border-zinc-600 dark:bg-zinc-800 dark:text-zinc-200"
                  />
                </FieldRow>
                <p className="text-xs text-zinc-500 dark:text-zinc-400">
                  Current:{" "}
                  {effective === null ? (
                    <span>no override (Claude Code default)</span>
                  ) : (
                    <code>{effective}</code>
                  )}
                </p>
                {invalid && check !== undefined && (
                  <p
                    id={`role-${key}-error`}
                    aria-live="polite"
                    className="text-xs font-medium text-red-600 dark:text-red-400"
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
