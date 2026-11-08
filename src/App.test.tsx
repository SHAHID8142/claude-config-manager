import { describe, expect, it } from "vitest";
import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import App from "./App";
import { createMockIpc } from "./lib/ipc-mock";
import { providerSummaryFixtures } from "./lib/fixtures/providers";

function mockWithProviders() {
  return createMockIpc({
    handlers: { list_providers: () => Promise.resolve(providerSummaryFixtures) },
  });
}

describe("App shell (T12/T14 regression)", () => {
  it("renders the app title and all PRD sections", async () => {
    render(<App ipc={mockWithProviders()} />);
    expect(screen.getByRole("heading", { name: "Claude Config Manager" })).toBeInTheDocument();
    for (const section of ["Active configuration", "Provider", "Model", "Actions"]) {
      expect(screen.getByRole("region", { name: section })).toBeInTheDocument();
    }
    expect(screen.getByText("mode: not configured")).toBeInTheDocument();
  });

  it("keeps unwired controls disabled with honest placeholders", async () => {
    render(<App ipc={mockWithProviders()} />);
    expect(screen.getByLabelText("API key", { selector: "input" })).toBeDisabled();
    expect(screen.getByLabelText("Model", { selector: "input" })).toBeDisabled();
    expect(screen.getByRole("button", { name: "Test Connection" })).toBeDisabled();
    expect(screen.getByRole("button", { name: "Save & Apply" })).toBeDisabled();
    // Show/hide toggle follows the field's disabled state (T20)
    expect(screen.getByRole("button", { name: "Show" })).toBeDisabled();
  });

  it("enables the masked API key field once a provider is selected (T20)", async () => {
    render(<App ipc={mockWithProviders()} />);
    const select = await screen.findByLabelText("Provider", { selector: "select" });
    await userEvent.selectOptions(select, "anthropic");
    const keyInput = screen.getByLabelText("API key", { selector: "input" });
    expect(keyInput).toBeEnabled();
    expect(keyInput.getAttribute("type")).toBe("password");
    const showBtn = screen.getByRole("button", { name: "Show" });
    expect(showBtn).toBeEnabled();
    await userEvent.click(showBtn);
    expect(screen.getByRole("button", { name: "Hide" })).toHaveAttribute("aria-pressed", "true");
    const revealed = screen.getByLabelText("API key", { selector: "input" });
    expect(revealed.getAttribute("type")).toBe("text");
    await userEvent.type(revealed, "sk-test-fixture");
    expect((revealed as HTMLInputElement).value).toBe("sk-test-fixture");
  });

  it("links field hints via aria-describedby", async () => {
    render(<App ipc={mockWithProviders()} />);
    const apiKey = screen.getByLabelText("API key", { selector: "input" });
    expect(apiKey).toHaveAttribute("aria-describedby", "api-key-input-hint");
  });
});

describe("Provider selector (T18)", () => {
  it("loads and lists builtin providers from the catalog", async () => {
    const ipc = mockWithProviders();
    render(<App ipc={ipc} />);
    const select = await screen.findByLabelText("Provider", { selector: "select" });
    await waitFor(() => expect(select).toBeEnabled());
    const options = Array.from(select.querySelectorAll("option")).map((o) => o.textContent);
    expect(options).toEqual(["Select a provider", "Anthropic", "Custom…"]);
    expect(ipc.calls.map((c) => c.cmd)).toEqual(["list_providers"]);
  });

  it("selecting a preset shows its verified capability and direct-mode note", async () => {
    const ipc = mockWithProviders();
    render(<App ipc={ipc} />);
    const select = await screen.findByLabelText("Provider", { selector: "select" });
    await userEvent.selectOptions(select, "anthropic");
    expect(screen.getByText(/Connects directly — no bridge process needed/)).toBeInTheDocument();
    expect(screen.getByText(/Capability status: verified/)).toBeInTheDocument();
    expect(screen.getByRole("link", { name: "(source)" })).toHaveAttribute(
      "href",
      "https://platform.claude.com/docs/en/api/models-list",
    );
    // Preset endpoint shown read-only and disabled.
    const endpoint = screen.getByLabelText("Endpoint URL", { selector: "input" });
    expect(endpoint).toBeDisabled();
    expect(endpoint).toHaveValue("https://api.anthropic.com");
  });

  it("selecting Custom reveals custom name/endpoint entry", async () => {
    const ipc = mockWithProviders();
    render(<App ipc={ipc} />);
    const select = await screen.findByLabelText("Provider", { selector: "select" });
    await userEvent.selectOptions(select, "__custom__");
    const nameInput = screen.getByLabelText("Custom provider name", { selector: "input" });
    const endpointInput = screen.getByLabelText("Custom endpoint URL", { selector: "input" });
    expect(nameInput).toBeEnabled();
    expect(endpointInput).toBeEnabled();
    await userEvent.type(nameInput, "My gateway");
    await userEvent.type(endpointInput, "https://gw.example.com");
    expect(nameInput).toHaveValue("My gateway");
    expect(endpointInput).toHaveValue("https://gw.example.com");
    // https remote → no warning shown
    expect(screen.queryByText(/insecure connection/i)).not.toBeInTheDocument();
  });

  it("warns insecure http endpoints live while typing (T19)", async () => {
    const ipc = mockWithProviders();
    render(<App ipc={ipc} />);
    const select = await screen.findByLabelText("Provider", { selector: "select" });
    await userEvent.selectOptions(select, "__custom__");
    const endpointInput = screen.getByLabelText("Custom endpoint URL", { selector: "input" });
    await userEvent.type(endpointInput, "http://evil.example.com");
    const warning = screen.getByText(/Refused: credentials would be sent over an insecure/i);
    expect(warning).toHaveAttribute("aria-live", "polite");
  });

  it("notes local endpoints without failing them (T19)", async () => {
    const ipc = mockWithProviders();
    render(<App ipc={ipc} />);
    const select = await screen.findByLabelText("Provider", { selector: "select" });
    await userEvent.selectOptions(select, "__custom__");
    const endpointInput = screen.getByLabelText("Custom endpoint URL", { selector: "input" });
    await userEvent.type(endpointInput, "https://localhost:8080");
    expect(
      screen.getByText(/Local endpoint — API traffic stays on this machine/i),
    ).toBeInTheDocument();
  });

  it("shows an error banner when the catalog fails to load", async () => {
    const ipc = createMockIpc({
      failures: { list_providers: { kind: "not_available", message: "backend offline" } },
    });
    render(<App ipc={ipc} />);
    await waitFor(() => {
      expect(screen.getByRole("alert")).toBeInTheDocument();
    });
    expect(screen.getByText("backend offline")).toBeInTheDocument();
  });
});
