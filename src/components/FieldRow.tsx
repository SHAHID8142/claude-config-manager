/** Layout primitive: labelled row with optional hint text (a11y-first). */
interface FieldRowProps {
  label: string;
  htmlFor: string;
  hint?: string;
  children: React.ReactNode;
}

export function FieldRow({ label, htmlFor, hint, children }: FieldRowProps) {
  return (
    <div className="flex flex-col gap-1 py-2">
      <label htmlFor={htmlFor} className="text-sm font-medium text-zinc-800 dark:text-zinc-200">
        {label}
      </label>
      {children}
      {hint !== undefined && (
        <p id={`${htmlFor}-hint`} className="text-xs text-zinc-500 dark:text-zinc-400">
          {hint}
        </p>
      )}
    </div>
  );
}
