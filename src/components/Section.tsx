/** Card section container for the single-column layout. */
interface SectionProps {
  title: string;
  children: React.ReactNode;
}

export function Section({ title, children }: SectionProps) {
  return (
    <section aria-label={title} className="rounded-xl border border-line bg-surface px-4 py-3.5">
      <h2 className="mb-2.5 text-[13px] font-semibold text-ink-muted">{title}</h2>
      {children}
    </section>
  );
}
