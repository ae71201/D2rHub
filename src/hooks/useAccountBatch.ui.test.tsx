import { act, cleanup, renderHook } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { useAccounts } from "../store/accounts";
import { useAccountBatch } from "./useAccountBatch";
import type { AccountMeta } from "../store/types";

const mocks = vi.hoisted(() => ({ invoke: vi.fn(), visible: vi.fn(), minimized: vi.fn() }));
vi.mock("../platform/tauri", () => ({ invokeCommand: mocks.invoke }));
vi.mock("@tauri-apps/api/window", () => ({ getCurrentWindow: () => ({ isVisible: mocks.visible, isMinimized: mocks.minimized }) }));
const account = { id: "a", initialized: true, is_running: false, auth_mode: "token", region: "KR" } as AccountMeta;
beforeEach(() => {
  vi.useFakeTimers();
  useAccounts.setState({ accounts: [account] });
  mocks.visible.mockResolvedValue(true);
  mocks.minimized.mockResolvedValue(false);
  mocks.invoke.mockImplementation(async (command: string) => command === "inspect_account_launch_health"
    ? [{ account_id: "a", error: null }] : []);
});
afterEach(() => { cleanup(); vi.useRealTimers(); vi.clearAllMocks(); });
describe("batch runtime resource lifecycle", () => {
  it("uses one serial poll, caches health and cancels timers on unmount", async () => {
    const { result, unmount } = renderHook(() => useAccountBatch(true, false, null, false));
    await act(async () => {});
    expect(result.current.uncertain).toBe(false);
    await act(async () => { await vi.advanceTimersByTimeAsync(9000); });
    expect(mocks.invoke.mock.calls.filter(([cmd]) => cmd === "refresh_account_running_state")).toHaveLength(4);
    expect(mocks.invoke.mock.calls.filter(([cmd]) => cmd === "inspect_account_launch_health")).toHaveLength(1);
    unmount();
    const count = mocks.invoke.mock.calls.length;
    await vi.advanceTimersByTimeAsync(9000);
    expect(mocks.invoke).toHaveBeenCalledTimes(count);
  });
  it("does not scan processes in a hidden or minimized window", async () => {
    mocks.visible.mockResolvedValue(false);
    const { result } = renderHook(() => useAccountBatch(true, false, null, false));
    await act(async () => { await vi.advanceTimersByTimeAsync(6000); });
    expect(mocks.invoke).not.toHaveBeenCalled();
    expect(result.current.motionPaused).toBe(true);
    mocks.visible.mockResolvedValue(true);
    mocks.minimized.mockResolvedValue(true);
    await act(async () => { await vi.advanceTimersByTimeAsync(3000); });
    expect(mocks.invoke).not.toHaveBeenCalled();
  });
  it("preserves last known running state on scan failure and blocks new selection", async () => {
    useAccounts.setState({ accounts: [{ ...account, is_running: true }] });
    mocks.invoke.mockRejectedValue(new Error("IPC unavailable"));
    const { result } = renderHook(() => useAccountBatch(true, false, null, false));
    await act(async () => {});
    expect(result.current.uncertain).toBe(true);
    expect(useAccounts.getState().accounts[0].is_running).toBe(true);
    act(() => result.current.toggle(useAccounts.getState().accounts[0]));
    expect(result.current.selection.ids).toEqual([]);
  });
  it("prevents overlapping refresh requests and ignores completed health after disposal", async () => {
    let resolve!: (ids: string[]) => void;
    mocks.invoke.mockImplementation((cmd: string) => cmd === "refresh_account_running_state"
      ? new Promise<string[]>(done => { resolve = done; }) : Promise.resolve([]));
    const { result, unmount } = renderHook(() => useAccountBatch(true, false, null, false));
    await act(async () => {});
    act(() => { void result.current.refresh(); window.dispatchEvent(new Event("focus")); });
    await act(async () => { await vi.advanceTimersByTimeAsync(12000); });
    expect(mocks.invoke).toHaveBeenCalledTimes(1);
    unmount();
    await act(async () => { resolve([]); });
    expect(vi.getTimerCount()).toBe(0);
  });
});

describe("select all accounts", () => {
  it("selects only ready idle accounts in card order and explicitly replaces the scope without toggling a full selection off", async () => {
    const rows = [
      { ...account, id: "later", order: 3 },
      { ...account, id: "earlier", order: 1 },
      { ...account, id: "invalid", order: 2 },
      { ...account, id: "incomplete", initialized: false },
      { ...account, id: "unknown" },
      { ...account, id: "running", is_running: true },
    ];
    useAccounts.setState({ accounts: rows });
    mocks.invoke.mockImplementation(async (command: string) => command === "inspect_account_launch_health"
      ? rows.filter(row => row.id !== "unknown").map(row => ({ account_id: row.id, error: row.id === "invalid" ? "Missing Token" : null }))
      : ["running"]);
    const { result } = renderHook(() => useAccountBatch(true, false, null, false));
    await act(async () => {});
    expect(result.current.selectableIds.launch).toEqual(["earlier", "later"]);
    act(() => result.current.toggle(useAccounts.getState().accounts[0]));
    act(() => result.current.selectAll("launch"));
    expect(result.current.selection).toEqual({ mode: "launch", ids: ["earlier", "later"] });
    act(() => result.current.selectAll("close"));
    expect(result.current.selection).toEqual({ mode: "close", ids: ["running"] });
    act(() => result.current.selectAll("launch"));
    expect(result.current.selection).toEqual({ mode: "launch", ids: ["earlier", "later"] });
    act(() => result.current.selectAll("launch"));
    expect(result.current.selection).toEqual({ mode: "launch", ids: ["earlier", "later"] });
    act(() => result.current.clear());
    expect(result.current.selection).toEqual({ mode: null, ids: [] });
  });

  it("allows selecting running accounts with invalid launch configuration, but never selects while blocked", async () => {
    useAccounts.setState({ accounts: [{ ...account, is_running: true }] });
    mocks.invoke.mockImplementation(async (command: string) => command === "inspect_account_launch_health"
      ? [{ account_id: "a", error: "Missing Token" }] : ["a"]);
    const { result, rerender } = renderHook(({ busy, suspended, active }) => useAccountBatch(active, suspended, null, busy),
      { initialProps: { busy: false, suspended: false, active: true } });
    act(() => result.current.selectAll("close"));
    expect(result.current.selection.ids).toEqual([]);
    await act(async () => {});
    act(() => result.current.selectAll("close"));
    expect(result.current.selection.ids).toEqual(["a"]);
    act(() => result.current.clear());
    rerender({ busy: true, suspended: false, active: true });
    act(() => result.current.selectAll("close"));
    expect(result.current.selection.ids).toEqual([]);
    rerender({ busy: false, suspended: true, active: true });
    act(() => result.current.selectAll("close"));
    expect(result.current.selection.ids).toEqual([]);
    rerender({ busy: false, suspended: false, active: false });
    act(() => result.current.selectAll("close"));
    expect(result.current.selection.ids).toEqual([]);
  });

  it("invalidates old health immediately, rejects a stale reply and removes selections that become unhealthy", async () => {
    const { result } = renderHook(() => useAccountBatch(true, false, null, false));
    await act(async () => {});
    act(() => result.current.selectAll("launch"));
    let resolve!: (rows: { account_id: string; error: string | null }[]) => void;
    mocks.invoke.mockImplementation((command: string) => command === "inspect_account_launch_health"
      ? new Promise(done => { resolve = done; }) : Promise.resolve([]));
    act(() => useAccounts.setState({ accounts: [{ ...account, mod_args: "-mod old" }] }));
    expect(result.current.uncertain).toBe(true);
    expect(result.current.issue(useAccounts.getState().accounts[0])).toBeUndefined();
    await act(async () => {});
    act(() => useAccounts.setState({ accounts: [{ ...account, mod_args: "-mod new" }] }));
    mocks.invoke.mockImplementation(async (command: string) => command === "inspect_account_launch_health"
      ? [{ account_id: "a", error: "Mod missing" }] : []);
    await act(async () => { resolve([{ account_id: "a", error: null }]); });
    expect(result.current.uncertain).toBe(true);
    await act(async () => { await vi.advanceTimersByTimeAsync(0); });
    expect(result.current.uncertain).toBe(false);
    expect(result.current.selectableIds.launch).toEqual([]);
    expect(result.current.selection.ids).toEqual([]);
  });

  it("does not silently extend an existing selection when another account becomes ready", async () => {
    const { result } = renderHook(() => useAccountBatch(true, false, null, false));
    await act(async () => {});
    act(() => result.current.selectAll("launch"));
    mocks.invoke.mockImplementation(async (command: string) => command === "inspect_account_launch_health"
      ? [{ account_id: "a", error: null }, { account_id: "b", error: null }] : []);
    await act(async () => { useAccounts.setState({ accounts: [account, { ...account, id: "b" }] }); });
    expect(result.current.selection.ids).toEqual(["a"]);
    expect(result.current.selectableIds.launch).toEqual(["a", "b"]);
    act(() => result.current.selectAll("launch"));
    expect(result.current.selection.ids).toEqual(["a", "b"]);
  });

  it("clears with Escape from a selection checkbox while preserving selection during text editing", async () => {
    const { result } = renderHook(() => useAccountBatch(true, false, null, false));
    await act(async () => {});
    act(() => result.current.selectAll("launch"));
    const input = document.createElement("input");
    document.body.append(input);
    try {
      act(() => input.dispatchEvent(new KeyboardEvent("keydown", { key: "Escape", bubbles: true })));
      expect(result.current.selection.ids).toEqual(["a"]);
      input.type = "checkbox";
      act(() => input.dispatchEvent(new KeyboardEvent("keydown", { key: "Escape", bubbles: true })));
      expect(result.current.selection.ids).toEqual([]);
    } finally { input.remove(); }
  });
});
