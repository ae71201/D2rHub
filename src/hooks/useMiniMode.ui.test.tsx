import { cleanup, renderHook, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, expect, it, vi } from "vitest";
import { version } from "../../package.json";
import { useMiniMode } from "./useMiniMode";
const mocks = vi.hoisted(() => ({
  setSize: vi.fn(async () => {}),
  setPosition: vi.fn(async () => {}),
  monitor: { size: { width: 2560, height: 1440 }, scaleFactor: 1.5,
    workArea: { position: { x: -2560, y: 0 }, size: { width: 2560, height: 1380 } } },
}));
vi.mock("@tauri-apps/api/window", () => ({
  LogicalSize: class { constructor(public width: number, public height: number) {} },
  PhysicalPosition: class { constructor(public x: number, public y: number) {} },
  currentMonitor: async () => mocks.monitor,
  availableMonitors: async () => [mocks.monitor],
  getCurrentWindow: () => ({
    outerPosition: async () => ({ x: -2200, y: 100 }), innerSize: async () => ({ width: 1500, height: 900 }),
    scaleFactor: async () => 1.5, setMinSize: async () => {}, setPosition: mocks.setPosition,
    setSize: mocks.setSize, onMoved: async () => () => {}, onResized: async () => () => {},
    isMinimized: async () => false, setAlwaysOnTop: async () => {},
  }),
}));
vi.mock("../platform/tauri", () => ({ invokeCommand: vi.fn(async () => {}) }));
vi.mock("../components/ui/Toast", () => ({ showToast: vi.fn() }));
beforeEach(() => { localStorage.clear(); vi.clearAllMocks(); });
afterEach(cleanup);
const prefix = "d2rhub-main-layout-v1:";
it("resets an earlier version's saved size and mini mode using DPI-independent monitor proportions", async () => {
  localStorage.setItem(prefix + "version", "older");
  localStorage.setItem(prefix + "mode", "mini");
  localStorage.setItem(prefix + "full", JSON.stringify({ x: -2200, y: 100, width: 800, height: 600 }));
  const { result } = renderHook(() => useMiniMode(true, false));
  await waitFor(() => expect(result.current.busy).toBe(false));
  const size = mocks.setSize.mock.calls[0] as unknown as [{width: number; height: number}];
  expect(size[0].width).toBeCloseTo(2560 / 1.5 * .7);
  expect(size[0].height).toBeCloseTo(size[0].width * .58);
  expect(localStorage.getItem(prefix + "version")).toBe(version);
  expect(result.current.mini).toBe(false);
  expect((mocks.setPosition.mock.calls[0] as unknown as [{x:number}])[0].x).toBeLessThan(0);
});
it("restores a user-resized window on subsequent launches of the same version", async () => {
  localStorage.setItem(prefix + "version", version);
  localStorage.setItem(prefix + "full", JSON.stringify({ x: -2200, y: 100, width: 800, height: 600 }));
  const { result } = renderHook(() => useMiniMode(true, false));
  await waitFor(() => expect(result.current.busy).toBe(false));
  expect(mocks.setSize).toHaveBeenCalledWith(expect.objectContaining({ width: 800, height: 600 }));
});
