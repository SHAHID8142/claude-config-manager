import { Component, type ErrorInfo, type ReactNode } from "react";

interface Props {
  children: ReactNode;
}

interface State {
  error: Error | null;
}

/**
 * Last-resort guard so a render failure can never leave a blank window.
 *
 * The black-screen bug was an uncaught `TypeError` inside render: React tears the
 * whole tree down and the user gets an empty page with no explanation. Catching
 * here at least tells them what happened and offers a reload — a wrong-looking
 * screen is recoverable, a black one is not.
 */
export class ErrorBoundary extends Component<Props, State> {
  state: State = { error: null };

  static getDerivedStateFromError(error: Error): State {
    return { error };
  }

  componentDidCatch(error: Error, info: ErrorInfo): void {
    // No telemetry, no network: log locally so it is visible in the console.
    console.error("render failed:", error, info.componentStack);
  }

  render(): ReactNode {
    const { error } = this.state;
    if (error === null) return this.props.children;
    return (
      <main className="mx-auto flex min-h-screen max-w-2xl flex-col gap-4 bg-surface p-6">
        <h1 className="text-lg font-semibold text-ink">Something went wrong</h1>
        <p className="text-sm text-ink-muted">
          The interface hit an unexpected error and stopped. Your settings file was not changed by
          this.
        </p>
        <pre className="max-w-full overflow-x-auto rounded border border-line p-3 text-xs text-ink-muted">
          {error.message}
        </pre>
        <button
          type="button"
          onClick={() => window.location.reload()}
          className="btn btn-secondary self-start"
        >
          Reload
        </button>
      </main>
    );
  }
}
