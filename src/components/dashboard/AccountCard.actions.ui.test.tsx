import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, expect, it, vi } from "vitest";
import { AccountGridItem } from "./AccountCard";
import type { AccountMeta } from "../../store/types";

const mocks = vi.hoisted(() => ({ invoke: vi.fn(), toast: vi.fn(), flush: vi.fn() }));
vi.mock("../../platform/tauri", () => ({ invokeCommand: mocks.invoke }));
vi.mock("../ui/Toast", () => ({ showToast: mocks.toast }));
vi.mock("../../store/accounts", () => ({ useAccounts: () => ({ reinitializeAccount: vi.fn(), updateAccountPositions: vi.fn() }) }));
vi.mock("../../store/launch", () => ({ useLaunch: { setState: vi.fn() } }));
vi.mock("../../hooks/useAccountQuickSettings", () => ({ useAccountQuickSettings: () => ({ settings: { resolution: "1920x1080", fps: 60 }, loaded: false, loading: false, error: null, load: vi.fn().mockResolvedValue(undefined), update: vi.fn(), flush: mocks.flush }) }));
vi.mock("./AccountModEditor", () => ({ AccountModEditor: () => null }));
vi.mock("./AccountRegionSwitcher", () => ({ AccountRegionSwitcher: () => null }));

afterEach(() => { cleanup(); vi.clearAllMocks(); });
beforeEach(() => { mocks.flush.mockResolvedValue(undefined); });
const account: AccountMeta = { id: "a", display_name: "账号 A", mod_args: "", created_at: "2026-09-29T00:00:00Z", last_launched_at: null, last_reset_at: null, initialized: true, order: 0, is_running: true, auth_mode: "token", region: "CN" };
const props = () => ({ account, onRename: vi.fn(), onDelete: vi.fn(), onConfigure: vi.fn(), onLaunch: vi.fn(), onBattleNetOnly: vi.fn(), runtimeStatus: { issue: null, uncertain: false } });

it("focuses a running instance without launching it or expanding the card", async () => {
  mocks.invoke.mockResolvedValue(undefined);
  const p = props();
  render(<AccountGridItem {...p} />);
  fireEvent.click(screen.getByRole("button", { name: "聚焦" }));
  await waitFor(() => expect(mocks.invoke).toHaveBeenCalledWith("focus_game_window", { accountId: "a" }));
  expect(p.onLaunch).not.toHaveBeenCalled();
  expect(screen.getByRole("button", { name: /展开快捷配置/ }).getAttribute("aria-expanded")).toBe("false");
});

it("reports focus errors instead of treating them as a launch", async () => {
  mocks.invoke.mockRejectedValue(new Error("窗口已退出"));
  render(<AccountGridItem {...props()} />);
  fireEvent.click(screen.getByRole("button", { name: "聚焦" }));
  await waitFor(() => expect(mocks.toast).toHaveBeenCalledWith("error", expect.stringContaining("窗口已退出")));
});

it("continues initialization for an incomplete Token account", () => {
  const update = vi.fn();
  render(<AccountGridItem {...props()} account={{ ...account, initialized: false, is_running: false }} onUpdateToken={update} />);
  fireEvent.click(screen.getByRole("button", { name: "继续配置" }));
  expect(update).toHaveBeenCalledWith(expect.objectContaining({ id: "a" }));
});

it("keeps collapsed quick controls out of the accessibility tree", async () => {
  render(<AccountGridItem {...props()} />);
  expect(screen.queryByRole("combobox", { name: "账号 A 分辨率" })).toBeNull();
  fireEvent.click(screen.getByRole("button", { name: /展开快捷配置/ }));
  expect(screen.getByRole("combobox", { name: "账号 A 分辨率" })).toBeTruthy();
  fireEvent.click(screen.getByRole("button", { name: /收起快捷配置/ }));
  await waitFor(() => expect(screen.queryByRole("combobox", { name: "账号 A 分辨率" })).toBeNull());
});

it("keeps failed saves open and allows an explicit retry before closing", async () => {
  mocks.flush.mockRejectedValue(new Error("保存失败"));
  render(<AccountGridItem {...props()} />);
  fireEvent.click(screen.getByRole("button", { name: /展开快捷配置/ }));
  fireEvent.click(screen.getByRole("button", { name: "关闭快捷配置" }));
  await waitFor(() => expect(mocks.flush).toHaveBeenCalledOnce());
  expect(screen.getByRole("dialog", { name: /快捷配置/ })).toBeTruthy();
  mocks.flush.mockResolvedValue(undefined);
  fireEvent.click(screen.getByRole("button", { name: "关闭快捷配置" }));
  await waitFor(() => expect(screen.queryByRole("dialog", { name: /快捷配置/ })).toBeNull());
});

it("requires an explicit confirmation to delete an account", async () => {
  const p = props();
  render(<AccountGridItem {...p} />);
  fireEvent.click(screen.getByRole("button", { name: /更多操作/ }));
  fireEvent.click(screen.getByRole("button", { name: "删除账号…" }));
  expect(p.onDelete).not.toHaveBeenCalled();
  const confirm = await screen.findByRole("button", { name: "删除账号" });
  fireEvent.click(confirm);
  expect(p.onDelete).toHaveBeenCalledWith("a");
});

it("does not open quick settings when the card background is clicked", () => {
  const { container } = render(<AccountGridItem {...props()} />);
  fireEvent.click(container.querySelector('.account-tile')!);
  expect(screen.queryByRole("dialog")).toBeNull();
});

it("cancels rename without persisting its draft", () => {
  const p = props();
  render(<AccountGridItem {...p} />);
  fireEvent.click(screen.getByRole("button", { name: "账号 A" }));
  const field = screen.getByRole("textbox");
  fireEvent.change(field, { target: { value: "误输入" } });
  fireEvent.keyDown(field, { key: "Escape" });
  expect(p.onRename).not.toHaveBeenCalled();
  expect(screen.getByRole("button", { name: "账号 A" })).toBeTruthy();
});

it("focuses cancel in deletion confirmation and restores the actions trigger", async () => {
  render(<AccountGridItem {...props()} />);
  const trigger = screen.getByRole("button", { name: /更多操作/ });
  fireEvent.click(trigger);
  fireEvent.click(screen.getByRole("button", { name: "删除账号…" }));
  const cancel = screen.getByRole("button", { name: "取消" });
  await waitFor(() => expect(document.activeElement).toBe(cancel));
  fireEvent.click(cancel);
  await waitFor(() => expect(document.activeElement).toBe(trigger));
});

it("coalesces repeated close attempts while a save is pending", async () => {
  let finish!: () => void;
  mocks.flush.mockImplementation(() => new Promise<void>(resolve => { finish = resolve; }));
  render(<AccountGridItem {...props()} />);
  fireEvent.click(screen.getByRole("button", { name: /展开快捷配置/ }));
  const close = screen.getByRole("button", { name: "关闭快捷配置" });
  fireEvent.click(close);
  fireEvent.click(close);
  expect(mocks.flush).toHaveBeenCalledOnce();
  expect(screen.getByText("正在保存更改…")).toBeTruthy();
  finish();
  await waitFor(() => expect(screen.queryByRole("dialog", { name: /快捷配置/ })).toBeNull());
});

it("uses Escape for the innermost position editor before closing quick settings", async () => {
  render(<AccountGridItem {...props()} />);
  fireEvent.click(screen.getByRole("button", { name: /展开快捷配置/ }));
  const addPosition = screen.getByRole("button", { name: "+" });
  fireEvent.click(addPosition);
  const name = screen.getByRole("textbox", { name: "名称" });
  fireEvent.keyDown(name, { key: "Escape" });
  expect(screen.queryByRole("textbox", { name: "名称" })).toBeNull();
  const panel = screen.getByRole("dialog", { name: /快捷配置/ });
  expect(panel).toBeTruthy();
  expect(document.activeElement).toBe(addPosition);
  fireEvent.keyDown(addPosition, { key: "Escape" });
  await waitFor(() => expect(screen.queryByRole("dialog", { name: /快捷配置/ })).toBeNull());
});
