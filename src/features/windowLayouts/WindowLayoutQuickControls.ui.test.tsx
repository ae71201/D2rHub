import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { GlobalConfig } from "../../store/types";
import { useGlobalConfig } from "../../store/globalConfig";
import { WindowLayoutQuickControls } from "./WindowLayoutQuickControls";

const mocks = vi.hoisted(() => ({ restore: vi.fn(), toast: vi.fn(), patch: vi.fn() }));
vi.mock("./gateway", () => ({ restoreGameLayout: mocks.restore }));
vi.mock("../../components/ui/Toast", () => ({ showToast: mocks.toast }));
beforeEach(() => {
  vi.clearAllMocks();
  useGlobalConfig.setState({ saving: false, config: { app_language: "zh-CN", window_layout_enabled: true,
    active_window_layout_id: "desk", window_layouts: [{ id: "desk", name: "桌面", windows: [] }, { id: "other", name: "另一布局", windows: [] }] } as unknown as GlobalConfig,
    patch: mocks.patch });
  mocks.patch.mockImplementation(async patch => { const next = { ...useGlobalConfig.getState().config, ...patch } as GlobalConfig; useGlobalConfig.setState({ config: next }); return next; });
  mocks.restore.mockResolvedValue({ applied: ["account1"], failures: [] });
});
afterEach(cleanup);
function open() { fireEvent.click(screen.getByRole("button", { name: "窗口布局" })); }
describe("dashboard window layout quick panel", () => {
  it("discloses capsules from a compact trigger and can select no layout independently of the switch", async () => {
    render(<WindowLayoutQuickControls onOpenSettings={() => {}} />);
    expect(screen.queryByRole("group", { name: "选择布局" })).toBeNull();
    expect(screen.queryByRole("combobox")).toBeNull();
    open();
    expect(screen.getByRole("dialog", { name: "窗口布局" })).toBeTruthy();
    expect(screen.getByRole("button", { name: "桌面 0 开" }).getAttribute("aria-pressed")).toBe("true");
    fireEvent.click(screen.getByRole("button", { name: "不使用布局" }));
    await waitFor(() => expect(mocks.patch).toHaveBeenCalledWith({ active_window_layout_id: null }));
    expect(useGlobalConfig.getState().config?.window_layout_enabled).toBe(true);
    expect((screen.getByRole("button", { name: "恢复布局" }) as HTMLButtonElement).disabled).toBe(true);
    expect(mocks.restore).not.toHaveBeenCalled();
  });
  it("applies a selected capsule to running windows and restores with one click", async () => {
    render(<WindowLayoutQuickControls onOpenSettings={() => {}} />);
    open();
    fireEvent.click(screen.getByRole("button", { name: "另一布局 0 开" }));
    await waitFor(() => expect(mocks.restore).toHaveBeenCalledTimes(1));
    await waitFor(() => expect((screen.getByRole("button", { name: "恢复布局" }) as HTMLButtonElement).disabled).toBe(false));
    fireEvent.click(screen.getByRole("button", { name: "恢复布局" }));
    await waitFor(() => expect(mocks.restore).toHaveBeenCalledTimes(2));
  });
  it("toggles enablement on the outside button without opening the panel", async () => {
    render(<WindowLayoutQuickControls onOpenSettings={() => {}} />);
    fireEvent.click(screen.getByRole("switch", { name: "启用窗口布局" }));
    await waitFor(() => expect(mocks.patch).toHaveBeenCalledWith({ window_layout_enabled: false }));
    expect(screen.queryByRole("dialog")).toBeNull();
    expect(mocks.restore).not.toHaveBeenCalled();
  });
  it("reports native partial failures without claiming all windows restored", async () => {
    mocks.restore.mockResolvedValueOnce({ applied: [], failures: ["账号 A：窗口已关闭"] });
    render(<WindowLayoutQuickControls onOpenSettings={() => {}} />);
    open();
    fireEvent.click(screen.getByRole("button", { name: "恢复布局" }));
    await waitFor(() => expect(mocks.toast).toHaveBeenCalledWith("warning", "账号 A：窗口已关闭"));
    expect(screen.getByRole("alert").textContent).toBe("账号 A：窗口已关闭");
  });
  it("closes with Escape and opens full settings from the panel", async () => {
    const settings = vi.fn();
    render(<WindowLayoutQuickControls onOpenSettings={settings} />);
    open();
    fireEvent.keyDown(screen.getByRole("dialog"), { key: "Escape" });
    expect(screen.queryByRole("dialog")).toBeNull();
    open();
    fireEvent.click(screen.getByRole("button", { name: "管理布局" }));
    expect(settings).toHaveBeenCalledOnce();
    expect(screen.queryByRole("dialog")).toBeNull();
  });
});
