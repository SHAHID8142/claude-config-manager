import { describe, expect, it, vi } from "vitest";
import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { BridgeCard } from "./BridgeCard";
import { createMockIpc } from "../lib/ipc-mock";

const pin = {
  package: "@musistudio/claude-code-router",
  version: "3.1.3",
  tarballUrl: "https://registry.npmjs.org/x.tgz",
  integritySha512B64: "sha512-abc",
  minNodeMajor: 22,
  license: "MIT",
  licenseSource: "https://github.com/x",
  tarballBytes: 6529968,
};

const overview = (
  over: Partial<Parameters<typeof Object>["length"]> extends never ? object : object,
) => ({
  consented: false,
  installed: false,
  version: "3.1.3",
  nodeFound: true,
  nodeMajor: 26,
  running: false,
  pid: null,
  gatewayPort: 3456,
  ...over,
});

function mock(over: Record<string, unknown> = {}) {
  return createMockIpc({
    handlers: {
      bridge_consent_info: () => Promise.resolve(pin),
      bridge_overview: () => Promise.resolve(overview()),
      bridge_consent_record: () => Promise.resolve(undefined),
      bridge_install: () =>
        Promise.resolve({ version: "3.1.3", installDir: "/x", integrityVerified: true }),
      bridge_consent_withdraw: () => Promise.resolve(undefined),
      ...over,
    },
  });
}

describe("BridgeCard (T48/T52/T54)", () => {
  it("shows the consent panel with the full pin facts before anything installs", async () => {
    render(
      <BridgeCard
        ipc={mock()}
        providerId="nvidia-nim"
        providerName="NVIDIA NIM"
        model={null}
        onApplied={() => {}}
      />,
    );
    expect(await screen.findByText("Install the bridge?")).toBeInTheDocument();
    expect(screen.getByText("@musistudio/claude-code-router")).toBeInTheDocument();
    expect(screen.getByText("3.1.3 (pinned)")).toBeInTheDocument();
    expect(screen.getByText("MIT")).toBeInTheDocument();
    expect(screen.getByText("6.2 MiB, sha512-verified before install")).toBeInTheDocument();
    expect(screen.getByText("127.0.0.1 only")).toBeInTheDocument();
  });

  it("recording consent transitions to the ready state with install available", async () => {
    let consented = false;
    const ipc = createMockIpc({
      handlers: {
        bridge_consent_info: () => Promise.resolve(pin),
        bridge_consent_record: () => {
          consented = true;
          return Promise.resolve(undefined);
        },
        bridge_overview: () => Promise.resolve(overview({ consented })),
        bridge_install: () =>
          Promise.resolve({ version: "3.1.3", installDir: "/x", integrityVerified: true }),
      },
    });
    render(
      <BridgeCard
        ipc={ipc}
        providerId="nvidia-nim"
        providerName="NVIDIA NIM"
        model={null}
        onApplied={() => {}}
      />,
    );
    await userEvent.click(await screen.findByText("I understand — enable the bridge"));
    await waitFor(() => expect(screen.getByRole("button", { name: "Install" })).toBeEnabled());
  });

  it("surfaces backend errors without losing the state", async () => {
    const ipc = mock({
      bridge_consent_record: () => Promise.reject({ kind: "io", message: "disk full" }),
    });
    render(
      <BridgeCard
        ipc={ipc}
        providerId={null}
        providerName={null}
        model={null}
        onApplied={() => {}}
      />,
    );
    await userEvent.click(await screen.findByText("I understand — enable the bridge"));
    expect(await screen.findByText("Bridge error.")).toBeInTheDocument();
    expect(screen.getByText("disk full")).toBeInTheDocument();
  });
});
