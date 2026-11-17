import { useEffect, useState } from "react";
import { switchHistory, type IpcLike, type SwitchEvent } from "../lib/ipc";
import { HelpPopover } from "./HelpPopover";
const outcomeLabels: Record<SwitchEvent["outcome"], string> = {
  selected: "Selected after fallback",
  auth: "Authentication or access rejected",
  quota: "Credit or quota unavailable",
  rate_limit: "Rate limited",
  server_error: "Provider error",
  timeout: "Request timed out",
  network: "Network failure",
  request: "Request rejected",
};
export function SwitchHistory({ ipc }: { ipc: IpcLike }) {
  const [events, setEvents] = useState<SwitchEvent[]>([]);
  const [error, setError] = useState(false);
  useEffect(() => {
    let active = true;
    const update = () => {
      void switchHistory(ipc)
        .then((e) => {
          if (active) {
            setEvents(e);
            setError(false);
          }
        })
        .catch(() => {
          if (active) setError(true);
        });
    };
    update();
    const timer = window.setInterval(update, 5000);
    return () => {
      active = false;
      window.clearInterval(timer);
    };
  }, [ipc]);
  return (
    <details className="mt-3 text-[12px]">
      <summary>Saved switching history</summary>
      <p className="my-2 text-ink-muted">
        Last 100 failure and fallback events, retained after helper restart.{" "}
        <HelpPopover title="Saved switching history">
          Contains timestamps, provider identifiers and fixed outcome categories. It excludes
          prompts, responses, endpoints, API keys and key labels. Account-wide quota sharing cannot
          be inferred from a generic HTTP error.
        </HelpPopover>
      </p>
      {error ? (
        <p role="alert">History could not be read. Reopen this screen to retry.</p>
      ) : events.length ? (
        <ol className="space-y-2">
          {events
            .slice(-20)
            .reverse()
            .map((event, i) => (
              <li key={`${event.epoch}-${i}`}>
                <time dateTime={new Date(event.epoch * 1000).toISOString()}>
                  {new Date(event.epoch * 1000).toLocaleString()}
                </time>{" "}
                · {event.providerId} · {outcomeLabels[event.outcome]}
              </li>
            ))}
        </ol>
      ) : (
        <p>No saved switching events yet.</p>
      )}
    </details>
  );
}
