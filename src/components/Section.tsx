/**
 * One settings row: a label column (title + one sentence of plain description)
 * beside the panel that holds the controls. This is the hierarchy the screen
 * needs — every control is introduced by what it changes, and the columns give
 * the eye one left edge down the whole page instead of nested cards.
 */
interface SectionProps {
  title: string;
  /** One sentence, 12-14px grey: what this row changes. */
  description?: string;
  children: React.ReactNode;
}

export function Section({ title, description, children }: SectionProps) {
  return (
    <section
      aria-label={title}
      className="grid gap-x-8 gap-y-2 border-t border-line pt-5 first:border-t-0 first:pt-0 sm:grid-cols-[168px_minmax(0,1fr)]"
    >
      <div className="pt-0.5">
        <h2 className="text-[14px] leading-[1.3] font-semibold text-ink">{title}</h2>
        {description !== undefined && (
          <p className="mt-1.5 text-[12px] leading-[1.5] text-ink-muted">{description}</p>
        )}
      </div>
      <div className="min-w-0">{children}</div>
    </section>
  );
}
