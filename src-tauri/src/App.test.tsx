describe("Effective status + override warnings (T34)", () => {
  it("shows first-run state when no settings file exists", async () => {
    const ipc = mockWithProviders({
      ...statusFixture,
      settingsFound: false,
      model: null,
      apiKeyHelper: null,
    });
    render(<App ipc={ipc} />);
    expect(await screen.findByText(/No settings file yet/)).toBeInTheDocument();
    expect(await screen.findByText(/mode: not configured/)).toBeInTheDocument();
  });

  it("warns when apiKeyHelper is shadowed by a plaintext env key", async () => {
    const ipc = mockWithProviders({
      ...statusFixture,
      apiKeyPresent: true,
      warnings: [
        { helperShadowedByEnv: null },
        { plaintextKeyInEnv: { which: "ANTHROPIC_API_KEY" } },
      ],
    });
    render(<App ipc={ipc} />);
    expect(
      await screen.findByText(/apiKeyHelper is shadowed by a plaintext env credential/),
    ).toBeInTheDocument();
    expect(await screen.findByText(/ANTHROPIC_API_KEY is stored in plaintext/)).toBeInTheDocument();
    expect(await screen.findByText(/mode: direct \(plaintext env\)/)).not.toBeNull();
  });

  it("warns on model conflict with env override", async () => {
    const ipc = mockWithProviders({
      ...statusFixture,
      model: "claude-x",
      envModel: "claude-y",
      warnings: [{ modelConflict: { settingModel: "claude-x", envModel: "claude-y" } }],
    });
    render(<App ipc={ipc} />);
    expect(
      await screen.findByText(
        /Model conflict: settings say "claude-x" but ANTHROPIC_MODEL says "claude-y" \(env wins\)/,
      ),
    ).toBeInTheDocument();
  });

  it("discloses current endpoint without treating it as a secret", async () => {
    const ipc = mockWithProviders({
      ...statusFixture,
      envBaseUrl: "http://127.0.0.1:31415",
    });
    render(<App ipc={ipc} />);
    expect(await screen.findByText(/Endpoint:/)).toBeInTheDocument();
    expect(screen.getByText("http://127.0.0.1:31415")).toBeInTheDocument();
  });

  it("refreshes status after a successful apply", async () => {
    let statusCallCount = 0;
    const ipc = createMockIpc({
      handlers: {
        list_providers: () => Promise.resolve(providerSummaryFixtures),
        read_status: () => {
          statusCallCount += 1;
          return Promise.resolve(statusFixture);
        },
        save_and_apply: () =>
          Promise.resolve({
            settingsPath: "/tmp/x",
            backupFileName: "settings-backup-1.json",
            helperPath: "/tmp/h",
            appliedKeys: ["model"],
          }),
      },
    });
    render(<App ipc={ipc} />);
    const select = await screen.findByLabelText("Provider", { selector: "select" });
    await waitFor(() => expect(statusCallCount).toBe(1));
    await userEvent.selectOptions(select, "anthropic");
    await userEvent.type(screen.getByLabelText("API key", { selector: "input" }), "sk-x");
    await userEvent.click(screen.getByRole("button", { name: "Save & Apply" }));
    await waitFor(() => expect(statusCallCount).toBeGreaterThanOrEqual(2));
  });
});
