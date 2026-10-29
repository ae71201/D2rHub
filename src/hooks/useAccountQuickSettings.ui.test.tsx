import { act, cleanup, renderHook, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, expect, it, vi } from "vitest";
const platform = vi.hoisted(() => ({ invokeCommand: vi.fn(), emitEvent: vi.fn(), listenEvent: vi.fn(async () => () => {}) }));
vi.mock("../platform/tauri", () => platform);
import { useAccountQuickSettings } from "./useAccountQuickSettings";
afterEach(cleanup);
beforeEach(() => {
  vi.clearAllMocks(); platform.emitEvent.mockResolvedValue(undefined);
  platform.invokeCommand.mockImplementation(async (cmd: string) => cmd === "get_account_settings" ? { "Framerate Cap": 30, "Texture Quality": 3 } : {});
});
it("sends only the edited field, not a full settings snapshot", async () => {
  const { result } = renderHook(() => useAccountQuickSettings("one", true));
  await waitFor(() => expect(result.current.loaded).toBe(true));
  act(() => result.current.update({ fps: 60 }));
  await act(async () => { await result.current.flush(); });
  expect(platform.invokeCommand).toHaveBeenCalledWith("patch_account_settings", { accountId: "one", patch: { "Framerate Cap": 60 } });
  expect(platform.invokeCommand.mock.calls.filter(([cmd]) => cmd === "get_account_settings")).toHaveLength(1);
});
it("does not retry an already committed save when its notification fails", async () => {
  platform.emitEvent.mockRejectedValue(new Error("event unavailable"));
  vi.spyOn(console, "warn").mockImplementation(() => {});
  const { result } = renderHook(() => useAccountQuickSettings("one", true));
  await waitFor(() => expect(result.current.loaded).toBe(true));
  act(() => result.current.update({ fps: 60 }));
  await act(async () => { await result.current.flush(); await result.current.flush(); });
  expect(result.current.error).toBeNull();
  expect(platform.invokeCommand.mock.calls.filter(([cmd]) => cmd === "patch_account_settings")).toHaveLength(1);
});
