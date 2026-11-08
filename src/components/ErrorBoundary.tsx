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
      <main className="mx-auto flex min-h-screen max-w-2xl flex-col gap-4 bg-zinc-50 p-6 dark:bg-zinc-950">
        <h1 className="text-lg font-semibold text-zinc-900 dark:text-zinc-100">
          Something went wrong
        </h1>
        <p className="text-sm text-zinc-700 dark:text-zinc-300">
          The interface hit an unexpected error and stopped. Your settings file was not changed by
          this.
        </p>
        <pre className="max-w-full overflow-x-auto rounded border border-zinc-300 p-3 text-xs text-zinc-600 dark:border-zinc-700 dark:text-zinc-400">
          {error.message}
        </pre>
        <button
          type="button"
          onClick={() => window.location.reload()}
          className="self-start rounded-md bg-zinc-900 px-3 py-1.5 text-sm font-medium text-white dark:bg-zinc-100 dark:text-zinc-900"
        >
          Reload
        </button>
      </main>
    );
  }
}
