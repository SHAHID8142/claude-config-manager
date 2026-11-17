import { useState } from "react";
import { describe, expect, it, vi } from "vitest";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { RecoverySelector, type BackupPreview } from "./RecoverySelector";

const recent: BackupPreview = {
  fileName: "settings-backup-1794890788644002000.json",
  sizeBytes: 407,
  createdEpoch: 1794890788,
  valid: true,
  detail: null,
  model: "nvidia/nemotron-3-ultra-550b-a55b",
  baseUrl: "https://user:secret@example.com/private-token?api_key=secret#secret",
  helperPresent: true,
};
const older: BackupPreview = {
  ...recent,
  fileName: "settings-backup-1794890788134345000.json",
  model: "backup-model",
  baseUrl: null,
  helperPresent: false,
};
const corrupt: BackupPreview = {
  ...recent,
  fileName: "settings-backup-1794890789000000000.json",
  valid: false,
  detail: "Invalid JSON at line 1, column 1.",
  model: null,
  baseUrl: null,
  helperPresent: null,
};

describe("RecoverySelector", () => {
  it("previews the selected snapshot without exposing private parts of its URL", () => {
    const onSelect = vi.fn();
    const { container } = render(
      <RecoverySelector backups={[recent, older]} selected={recent.fileName} onSelect={onSelect} />,
    );
    expect(screen.getByRole("combobox", { name: "Settings backup" })).toHaveValue(recent.fileName);
    expect(screen.getByText(recent.model!)).toBeInTheDocument();
    expect(screen.getByText("https://example.com")).toBeInTheDocument();
    expect(container.innerHTML).not.toMatch(/secret|private-token|api_key/);
    expect(container.querySelector("time")).toHaveAttribute(
      "datetime",
      new Date(recent.createdEpoch * 1000).toISOString(),
    );
    expect(screen.getByText("Found on this Mac")).toBeInTheDocument();
    expect(onSelect).not.toHaveBeenCalled();
  });

  it("uses a native keyboard-focusable select and reports only deliberate valid selection", async () => {
    const onSelect = vi.fn();
    function Controlled() {
      const [selected, setSelected] = useState<string | null>(recent.fileName);
      return (
        <RecoverySelector
          backups={[recent, corrupt, older]}
          selected={selected}
          onSelect={(name) => {
            onSelect(name);
            setSelected(name);
          }}
        />
      );
    }
    const user = userEvent.setup();
    render(<Controlled />);
    const select = screen.getByRole("combobox", { name: "Settings backup" });
    expect(select.tagName).toBe("SELECT");
    select.focus();
    expect(select).toHaveFocus();
    expect(select).toHaveAccessibleDescription(/requires a separate confirmation/);
    await user.selectOptions(select, older.fileName);
    expect(onSelect).toHaveBeenLastCalledWith(older.fileName);
    expect(screen.getByText("backup-model")).toBeInTheDocument();
    expect(screen.getByText("Missing on this Mac — review before restoring")).toBeInTheDocument();
    expect(screen.getByText("Native Anthropic (no endpoint override)")).toBeInTheDocument();
    await user.selectOptions(select, corrupt.fileName);
    expect(onSelect).toHaveBeenCalledTimes(1);
    expect(select).toHaveValue(older.fileName);
  });

  it("keeps invalid snapshots visible with their reasons and never selects one automatically", () => {
    const onSelect = vi.fn();
    render(<RecoverySelector backups={[corrupt, recent]} selected={null} onSelect={onSelect} />);
    expect(screen.getByRole("option", { name: `${corrupt.fileName} — invalid` })).toBeDisabled();
    expect(screen.getByText(corrupt.detail!)).toBeInTheDocument();
    expect(screen.getByRole("combobox", { name: "Settings backup" })).toHaveValue("");
    expect(screen.queryByRole("heading", { name: "Selected backup" })).not.toBeInTheDocument();
    expect(onSelect).not.toHaveBeenCalled();
  });

  it("distinguishes loading, a read error, and no valid backups", () => {
    const props = { selected: null, onSelect: vi.fn() };
    const { rerender } = render(<RecoverySelector {...props} backups={null} />);
    expect(screen.getByRole("combobox", { name: "Settings backup" })).toBeDisabled();
    expect(screen.getByText("Reading backup previews…", { selector: "p" })).toBeInTheDocument();
    rerender(<RecoverySelector {...props} backups={[]} error="Backup directory cannot be read." />);
    expect(screen.getByRole("alert")).toHaveTextContent("Backup directory cannot be read.");
    expect(screen.getByText("No valid backups are available to restore.")).toBeInTheDocument();
    rerender(<RecoverySelector {...props} backups={[corrupt]} />);
    expect(screen.queryByRole("alert")).not.toBeInTheDocument();
    expect(screen.getByRole("combobox", { name: "Settings backup" })).toBeDisabled();
    expect(screen.getByText(corrupt.detail!)).toBeInTheDocument();
  });

  it("flags a removed or invalid parent selection without printing an unsafe endpoint", () => {
    const { rerender } = render(
      <RecoverySelector
        backups={[corrupt, recent]}
        selected={corrupt.fileName}
        onSelect={() => {}}
      />,
    );
    const select = screen.getByRole("combobox", { name: "Settings backup" });
    expect(select).toHaveAttribute("aria-invalid", "true");
    expect(select).toHaveAccessibleDescription(/selected backup is invalid/);
    rerender(<RecoverySelector backups={[recent]} selected="removed.json" onSelect={() => {}} />);
    expect(select).toHaveAccessibleDescription(/no longer available/);
    rerender(
      <RecoverySelector
        backups={[{ ...recent, baseUrl: "javascript:secret" }]}
        selected={recent.fileName}
        onSelect={() => {}}
      />,
    );
    expect(screen.getByText("Endpoint unavailable")).toBeInTheDocument();
    expect(screen.queryByText("javascript:secret")).not.toBeInTheDocument();
  });

  it("keeps IDs unique when a recovery confirmation and maintenance view coexist", () => {
    render(
      <>
        <RecoverySelector backups={[recent]} selected={null} onSelect={() => {}} />
        <RecoverySelector backups={[recent]} selected={null} onSelect={() => {}} />
      </>,
    );
    const selects = screen.getAllByRole("combobox", { name: "Settings backup" });
    expect(selects[0].id).not.toBe(selects[1].id);
    expect(screen.getAllByRole("button", { name: "About Settings backup" })).toHaveLength(2);
  });
});
