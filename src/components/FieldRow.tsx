/** Layout primitive: labelled row with optional hint text (a11y-first).
 * When a hint is present it is linked to the control via
 * aria-describedby so screen readers announce it with the field (T14). */
import { cloneElement, isValidElement } from "react";

interface FieldRowProps {
  label: string;
  htmlFor: string;
  hint?: string;
  children: React.ReactNode;
}

export function FieldRow({ label, htmlFor, hint, children }: FieldRowProps) {
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
    <div className="flex flex-col gap-1.5 py-2">
      <label htmlFor={htmlFor} className="text-[13px] font-medium text-ink">
        {label}
      </label>
      {control}
      {hintId !== undefined && (
        <p id={hintId} className="text-[12px] leading-snug text-ink-muted">
          {hint}
        </p>
      )}
    </div>
  );
}
