import type { ReactNode } from "react";
import { HelpPopover } from "./HelpPopover";

/**
 * One settings row: a label column (title + one short sentence) beside the
 * panel that holds the controls. Long explanations belong in the optional
 * help popover — the working screen stays scannable (distill pass).
 */
interface SectionProps {
  title: string;
  /** One short sentence, ≤90 chars: what this row changes. */
  description?: string;
  /** Long-form explanation, shown on demand beside the title. */
  help?: { title: string; children: ReactNode };
  children: React.ReactNode;
}

export function Section({ title, description, help, children }: SectionProps) {
  return (
    <section
      aria-label={title}
      className="grid gap-x-8 gap-y-2 border-t border-line pt-5 first:border-t-0 first:pt-0 sm:grid-cols-[168px_minmax(0,1fr)]"
    >
      <div className="pt-0.5">
        <h2 className="flex items-center gap-1.5 text-[14px] leading-[1.3] font-semibold text-ink">
          {title}
          {help !== undefined && (
            <HelpPopover title={help.title} label={`About ${title.toLowerCase()}`}>
              {help.children}
            </HelpPopover>
          )}
        </h2>
        {description !== undefined && (
          <p className="mt-1.5 text-[12px] leading-[1.5] text-ink-muted">{description}</p>
        )}
      </div>
      <div className="min-w-0">{children}</div>
    </section>
  );
}
