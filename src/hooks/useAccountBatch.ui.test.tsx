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
