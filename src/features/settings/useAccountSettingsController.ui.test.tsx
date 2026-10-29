import { act, renderHook, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";

import type { AccountMeta } from "../../store/types";

const platform = vi.hoisted(() => ({
  invokeCommand: vi.fn(),
  emitEvent: vi.fn(async () => undefined),
  listenEvent: vi.fn(async () => () => undefined),
}));

vi.mock("../../platform/tauri", () => platform);
vi.mock("../../components/ui/Toast", () => ({ showToast: vi.fn() }));

import { useAccountSettingsController } from "./useAccountSettingsController";

const accounts = [{
  id: "one",
  display_name: "Main",
  initialized: true,
  window_x: 12,
  window_y: 32,
}] as AccountMeta[];

beforeEach(() => {
  vi.clearAllMocks();
  platform.invokeCommand.mockImplementation(async (command: string, payload?: { patch?: Record<string, unknown> }) => {
    if (command === "get_account_settings") return { "Sound Volume": 60 };
    if (command === "patch_account_settings") return { "Sound Volume": 60, ...payload?.patch };
    return null;
  });
});

describe("account settings controller", () => {
  it("loads the selected account draft and game settings", async () => {
    const { result } = renderHook(() => useAccountSettingsController({
      accounts,
      loadAccounts: vi.fn(async () => undefined),
      renameAccount: vi.fn(async () => true),
    }));

    act(() => result.current.setSelectedAccountId("one"));

    await waitFor(() => expect(result.current.gameSettings).toEqual({ "Sound Volume": 60 }));
    expect(result.current.accountNicknameDraft).toBe("Main");
    expect(result.current.accountWinXDraft).toBe(12);
    expect(result.current.accountWinYDraft).toBe(32);
    expect(result.current.accountHasChanges).toBe(false);
  });

  it("persists changed game settings through the existing account command", async () => {
    const loadAccounts = vi.fn(async () => undefined);
    const { result } = renderHook(() => useAccountSettingsController({
      accounts,
      loadAccounts,
      renameAccount: vi.fn(async () => true),
    }));
    act(() => result.current.setSelectedAccountId("one"));
    await waitFor(() => expect(result.current.gameSettingsLoading).toBe(false));

    act(() => result.current.updateGameSetting("Sound Volume", 80));
    expect(result.current.accountHasChanges).toBe(true);

    await act(async () => {
      expect(await result.current.saveAccount(true)).toBe(true);
    });

    expect(platform.invokeCommand).toHaveBeenCalledWith("patch_account_settings", {
      accountId: "one",
      patch: { "Sound Volume": 80 },
    });
    expect(platform.emitEvent).toHaveBeenCalledWith("account-settings-updated", { accountId: "one" });
    expect(loadAccounts).toHaveBeenCalled();
  });
});

it("merges only edited fields and preserves edits made while saving", async () => {
  let finish!: (value: Record<string, unknown>) => void;
  platform.invokeCommand.mockImplementation(async (command: string) => {
    if (command === "get_account_settings") return { "Sound Volume": 60, "Texture Quality": 1 };
    if (command === "patch_account_settings") return new Promise(resolve => { finish = resolve; });
    return null;
  });
  const { result } = renderHook(() => useAccountSettingsController({ accounts, loadAccounts: vi.fn(async () => {}), renameAccount: vi.fn(async () => true) }));
  act(() => result.current.setSelectedAccountId("one"));
  await waitFor(() => expect(result.current.gameSettings["Sound Volume"]).toBe(60));
  act(() => result.current.updateGameSetting("Sound Volume", 80));
  let save!: Promise<boolean>;
  act(() => { save = result.current.saveAccount(true); });
  await waitFor(() => expect(finish).toBeDefined());
  expect(platform.invokeCommand).toHaveBeenCalledWith("patch_account_settings", { accountId: "one", patch: { "Sound Volume": 80 } });
  act(() => result.current.updateGameSetting("Sound Volume", 90));
  await act(async () => { finish({ "Sound Volume": 80, "Texture Quality": 3 }); await save; });
  expect(result.current.gameSettings).toEqual({ "Sound Volume": 90, "Texture Quality": 3 });
  expect(result.current.accountHasChanges).toBe(true);
});

it("does not save another account's draft after an asynchronous rename", async () => {
  let renamed!: (v: boolean) => void;
  const renameAccount = vi.fn(() => new Promise<boolean>(resolve => { renamed = resolve; }));
  platform.invokeCommand.mockImplementation(async (command: string, payload?: { accountId: string; patch?: Record<string, unknown> }) => {
    if (command === "get_account_settings") return { "Sound Volume": payload?.accountId === "one" ? 60 : 20 };
    if (command === "patch_account_settings") return payload?.patch;
    return null;
  });
  const twoAccounts = [...accounts, { ...accounts[0], id: "two", display_name: "Other" }];
  const { result } = renderHook(() => useAccountSettingsController({ accounts: twoAccounts, loadAccounts: vi.fn(async () => {}), renameAccount }));
  act(() => result.current.setSelectedAccountId("one"));
  await waitFor(() => expect(result.current.gameSettings["Sound Volume"]).toBe(60));
  act(() => { result.current.updateGameSetting("Sound Volume", 80); result.current.setAccountNicknameDraft("Renamed"); });
  let save!: Promise<boolean>;
  act(() => { save = result.current.saveAccount(true); });
  await waitFor(() => expect(renamed).toBeDefined());
  act(() => result.current.setSelectedAccountId("two"));
  await waitFor(() => expect(result.current.gameSettings["Sound Volume"]).toBe(20));
  act(() => result.current.updateGameSetting("Sound Volume", 99));
  await act(async () => { renamed(true); await save; });
  expect(platform.invokeCommand).toHaveBeenCalledWith("patch_account_settings", { accountId: "one", patch: { "Sound Volume": 80 } });
  expect(result.current.gameSettings["Sound Volume"]).toBe(99);
  expect(result.current.accountHasChanges).toBe(true);
});
