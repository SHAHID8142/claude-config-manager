import type { ReactNode } from "react";
import { HelpPopover } from "./HelpPopover";
export interface TooltipProps {
  content: ReactNode;
  label?: string;
  className?: string;
}
export function Tooltip({ content, label = "More information", className = "" }: TooltipProps) {
  return (
    <span className={className}>
      <HelpPopover title={label} label={label}>
        {content}
      </HelpPopover>
    </span>
  );
}
