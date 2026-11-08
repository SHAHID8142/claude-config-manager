import "@testing-library/jest-dom/vitest";
import { cleanup } from "@testing-library/react";
import { afterEach } from "vitest";

// Vitest runs without `globals: true`, so Testing Library's automatic
// cleanup hook never registers — do it explicitly to prevent DOM leakage
// between tests.
afterEach(() => {
  cleanup();
});
