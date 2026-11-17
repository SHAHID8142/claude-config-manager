/** Layout primitive: labelled row with optional hint text (a11y-first).
 * When a hint is present it is linked to the control via
 * aria-describedby so screen readers announce it with the field (T14). */
import { cloneElement, isValidElement } from "react";
import { HelpPopover } from "./HelpPopover";
import { FIELD_HELP } from "../lib/help";

interface FieldRowProps {
  label: string;
  htmlFor: string;
  hint?: string;
  help?: string;
  children: React.ReactNode;
}

export function FieldRow({ label, htmlFor, hint, help, children }: FieldRowProps) {
  const hintId = hint !== undefined ? `${htmlFor}-hint` : undefined;
  const control =
    isValidElement<{ "aria-describedby"?: string }>(children) && hintId
      ? cloneElement(children, {
          "aria-describedby": [children.props["aria-describedby"], hintId]
            .filter(Boolean)
            .join(" "),
        })
      : children;

  return (
    <div className="flex flex-col gap-2 py-2">
      <div className="flex items-center gap-1">
        <label htmlFor={htmlFor} className="text-[13px] font-medium text-ink">
          {label}
        </label>
        <HelpPopover title={label}>
          {help ??
            FIELD_HELP[label] ??
            hint ??
            `Set ${label.toLowerCase()} for this configuration. Changes take effect when you use the action in this section.`}
        </HelpPopover>
      </div>
      {control}
      {hintId !== undefined && (
        <p id={hintId} className="text-[12px] leading-snug text-ink-muted">
          {hint}
        </p>
      )}
    </div>
  );
}
