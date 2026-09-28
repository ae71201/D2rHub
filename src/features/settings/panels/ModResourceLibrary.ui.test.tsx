import { act, cleanup, render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { ModResourceLibrary } from "./ModResourceLibrary";
import { ModProcessorStatus } from "./ModProcessorStatus";
import type { TaskSnapshot } from "../../tasks/types";
import type { ModCapsuleController } from "../../modCapsules/useModCapsulePool";
const invoke = vi.hoisted(() => vi.fn());
const picker = vi.hoisted(() => vi.fn());
const subscribe = vi.hoisted(() => vi.fn());
vi.mock("../../../platform/tauri", () => ({ invokeCommand: invoke }));
vi.mock("@tauri-apps/plugin-dialog", () => ({ open: picker }));
vi.mock("../../tasks/taskSync", () => ({ subscribeBeforeReadingTasks: subscribe }));
const state = () => ({
  catalog: { release_url: "https://github.com/gjy991229/D2rHub/releases", assets: [
    { id: "processor", version: "1.4.0-beta.17", size: 3000000, url: "https://example.test/processor", game_data_version: null },
    { id: "LiteHub", version: "r1", size: 24000000, url: "https://example.test/mod", game_data_version: "93854" },
  ] },
  processor: { ready: false, installed_version: "1.3.3", recommended_version: "1.4.0-beta.17", installed_path: "C:\\old\\d2r-audio-mod.exe", install_directory: "C:\\User\\tools", legacy: true },
  mods_directory: "D:\\Game\\mods", game_data_version: "93854", warning: null,
});
const resourceTask = (overrides: Partial<TaskSnapshot> = {}): TaskSnapshot => ({
  revision: 1, task_id: 42, kind: "mod-resource-install", subject: "CN:LiteHub", conflict_key: null,
  state: "running", progress: 37, step: "verify", message: "正在校验", error_code: null,
  cancel_requested: false, retryable: false, retry_of: null, started_at_ms: 1, finished_at_ms: null,
  ...overrides,
});
beforeEach(() => { subscribe.mockReset().mockResolvedValue(() => {}); invoke.mockReset(); picker.mockReset(); invoke.mockImplementation(async (cmd: string) => cmd === "get_mod_resources" ? state() : { path: "C:\\User\\tools\\processor.exe" }); });
afterEach(cleanup);
describe("Mod resources", () => {
  it("trusts a fresh missing-resource status over a stale scanned catalog", async () => {
    invoke.mockResolvedValue({ ...state(), mods: [{ id: "LiteHub", installed_version: null,
      update_available: false, protected: false, message: "", reason_code: "not_installed" }] });
    const catalog = { pool: { capsules: [{ edition: "CN", origin: "scanned", name: "LiteHub" }] } } as ModCapsuleController;
    render(<ModResourceLibrary edition="CN" en={false} catalog={catalog} />);
    const card = (await screen.findByText("LiteHub")).closest("article")!;
    expect(within(card).getByRole("button", { name: "下载并安装" })).toHaveProperty("disabled", false);
  });
  it("does not leave stale task locks when the task subscription fails", async () => {
    subscribe.mockImplementation(async (_gateway, listener) => {
      listener(new Map([[42, resourceTask()]]));
      throw new Error("Task list unavailable");
    });
    render(<ModResourceLibrary edition="CN" en={false} />);
    const card = (await screen.findByText("LiteHub")).closest("article")!;
    await waitFor(() => expect(within(card).getByRole("button", { name: "下载并安装" })).toHaveProperty("disabled", false));
    expect(screen.getByRole("button", { name: "检查更新" })).toHaveProperty("disabled", false);
    expect(screen.getByText(/暂时无法读取任务进度/)).toBeTruthy();
  });
  it("presents a cancelled installation neutrally without hiding a later folder error", async () => {
    let publish!: (snapshot: ReadonlyMap<number, TaskSnapshot>) => void;
    let rejectInstall!: (error: Error) => void;
    subscribe.mockImplementation(async (_gateway, listener) => { publish = listener; return () => {}; });
    invoke.mockImplementation((cmd: string) => cmd === "get_mod_resources" ? Promise.resolve(state())
      : cmd === "install_mod_resource" ? new Promise((_resolve, reject) => { rejectInstall = reject; })
        : Promise.reject(new Error("Folder unavailable")));
    render(<ModResourceLibrary edition="CN" en={false} />);
    const card = (await screen.findByText("LiteHub")).closest("article")!;
    await userEvent.click(within(card).getByRole("button", { name: "下载并安装" }));
    await act(async () => {
      publish(new Map([[42, resourceTask({ state: "cancelled", message: "用户取消" })]]));
      rejectInstall(new Error("用户取消"));
    });
    expect(within(card).getByText("安装已取消，可以重新开始。")).toBeTruthy();
    expect(within(card).queryByRole("alert")).toBeNull();
    await userEvent.click(within(card).getByText("手动下载与安装位置"));
    await userEvent.click(within(card).getByRole("button", { name: "打开目录" }));
    expect((await within(card).findByRole("alert")).textContent).toContain("Folder unavailable");
  });
  it("reads locally on every visit and only refreshes online when requested", async () => {
    const first = render(<ModResourceLibrary edition="CN" en={false} />);
    await screen.findByText("LiteHub"); first.unmount();
    render(<ModResourceLibrary edition="CN" en={false} />);
    await screen.findByText("LiteHub");
    expect(invoke.mock.calls.filter(([cmd]) => cmd === "get_mod_resources").every(([, args]) => args.refresh === false)).toBe(true);
    await userEvent.click(screen.getByRole("button", { name: "检查更新" }));
    await waitFor(() => expect(invoke).toHaveBeenCalledWith("get_mod_resources", { edition: "CN", refresh: true }));
  });
  it("stops loading after an initial read failure and lets the user retry", async () => {
    invoke.mockRejectedValueOnce(new Error("资源目录暂时不可用"));
    render(<ModResourceLibrary edition="CN" en={false} />);

    expect((await screen.findByRole("alert")).textContent).toContain("资源目录暂时不可用");
    expect(screen.queryByText("正在检查本地资源…")).toBeNull();
    const retry = screen.getByRole("button", { name: "检查更新" }) as HTMLButtonElement;
    expect(retry.disabled).toBe(false);
    await userEvent.click(retry);

    expect(await screen.findByText("LiteHub")).toBeTruthy();
    expect(invoke).toHaveBeenLastCalledWith("get_mod_resources", { edition: "CN", refresh: true });
    expect(screen.queryByRole("alert")).toBeNull();
  });
  it("keeps a current processor quiet in the processing view", async () => {
    invoke.mockResolvedValue({ ...state(), processor: { ...state().processor, ready: true, update_available: false } });
    const ready = vi.fn();
    const view = render(<ModProcessorStatus edition="CN" en={false} onReady={ready} onManage={vi.fn()} />);
    await waitFor(() => expect(ready).toHaveBeenLastCalledWith(true));
    expect(view.container.textContent).toBe("");
    expect(invoke).toHaveBeenCalledTimes(1);
    expect(invoke).toHaveBeenCalledWith("get_mod_resources", { edition: "CN", refresh: false });
  });
  it("shows immediate feedback inside the clicked resource card before a task arrives", async () => {
    invoke.mockImplementation((cmd: string) => cmd === "get_mod_resources" ? Promise.resolve(state()) : new Promise(() => {}));
    render(<ModResourceLibrary edition="CN" en={false} />);
    const card = (await screen.findByText("LiteHub")).closest("article")!;
    await userEvent.click(within(card).getByRole("button", { name: "下载并安装" }));
    expect(within(card).getByText("正在准备…")).toBeTruthy();
    expect(within(card).getByRole("progressbar", { name: "LiteHub 进度" }).hasAttribute("aria-valuenow")).toBe(false);
    expect(within(card).getByRole("button", { name: "正在处理…" }).hasAttribute("disabled")).toBe(true);
    expect(within(screen.getByRole("article", { name: "Mod 加工器" })).queryByRole("progressbar")).toBeNull();
  });
  it("reattaches running progress to its resource after returning to the page", async () => {
    subscribe.mockImplementation(async (_gateway, listener) => {
      listener(new Map([[42, { task_id: 42, kind: "mod-resource-install", subject: "CN:LiteHub", state: "running", progress: 37, message: "正在校验", cancel_requested: false }]]));
      return () => {};
    });
    render(<ModResourceLibrary edition="CN" en={false} />);
    const card = (await screen.findByText("LiteHub")).closest("article")!;
    expect(within(card).getByRole("progressbar").getAttribute("aria-valuenow")).toBe("37");
    expect(within(card).getByText("正在校验")).toBeTruthy();
    expect(within(card).getByRole("button", { name: "取消" })).toBeTruthy();
  });
  it("updates the parent busy state when an observed background task starts and finishes", async () => {
    let publish!: (snapshot: ReadonlyMap<number, TaskSnapshot>) => void;
    subscribe.mockImplementation(async (_gateway, listener) => { publish = listener; return () => {}; });
    const onBusy = vi.fn();
    render(<ModResourceLibrary edition="CN" en={false} onBusy={onBusy} />);
    await screen.findByText("LiteHub");
    expect(onBusy).toHaveBeenLastCalledWith(false);

    await act(async () => publish(new Map([[42, resourceTask()]])));
    expect(onBusy).toHaveBeenLastCalledWith(true);
    expect((screen.getByRole("button", { name: "检查更新" }) as HTMLButtonElement).disabled).toBe(true);

    await act(async () => publish(new Map([[42, resourceTask({ revision: 2, state: "succeeded", progress: 100, message: "已安装" })]])));
    expect(onBusy).toHaveBeenLastCalledWith(false);
    expect(screen.queryByRole("progressbar")).toBeNull();
  });
  it("releases the parent busy state and subscription when leaving during a running task", async () => {
    const stop = vi.fn();
    subscribe.mockImplementation(async (_gateway, listener) => {
      listener(new Map([[42, resourceTask()]]));
      return stop;
    });
    const onBusy = vi.fn();
    const view = render(<ModResourceLibrary edition="CN" en={false} onBusy={onBusy} />);
    await screen.findByText("LiteHub");
    expect(onBusy).toHaveBeenLastCalledWith(true);

    view.unmount();
    expect(onBusy).toHaveBeenLastCalledWith(false);
    expect(stop).toHaveBeenCalledTimes(1);
  });
  it("shows a legacy processor version and installs the update in the managed location", async () => {
    const user = userEvent.setup(); render(<ModResourceLibrary edition="Global" en={false} />);
    await screen.findByText(/检测到旧版内置加工器/);
    for (const summary of screen.getAllByText("手动下载与安装位置")) await user.click(summary);
    expect(screen.getByText("C:\\User\\tools")).toBeTruthy();
    expect(screen.getByText("D:\\Game\\mods\\LiteHub")).toBeTruthy();
    await user.click(screen.getByRole("button", { name: "更新加工器" }));
    await waitFor(() => expect(invoke).toHaveBeenCalledWith("install_mod_resource", { edition: "Global", resourceId: "processor", localFile: null }));
  });
  it("blocks incompatible game packages without blocking processor installation", async () => {
    invoke.mockResolvedValue({ ...state(), game_data_version: "99999" });
    render(<ModResourceLibrary edition="CN" en={false} />);
    await screen.findByText("LiteHub");
    const card = screen.getByText("LiteHub").closest("article")!;
    expect((within(card).getByRole("button", { name: "下载并安装" }) as HTMLButtonElement).disabled).toBe(true);
    expect((screen.getByRole("button", { name: "更新加工器" }) as HTMLButtonElement).disabled).toBe(false);
  });
  it("restores a failed background installation after reopening the library", async () => {
    subscribe.mockImplementation(async (_gateway, listener) => {
      listener(new Map([[42, resourceTask({ state: "failed", message: "文件校验失败，请重试" })]]));
      return () => {};
    });
    const first = render(<ModResourceLibrary edition="CN" en={false} />);
    await screen.findByText("LiteHub");
    expect(screen.getByRole("alert").textContent).toContain("文件校验失败");
    first.unmount();
    render(<ModResourceLibrary edition="CN" en={false} />);
    const card = (await screen.findByText("LiteHub")).closest("article")!;
    expect(within(card).getByRole("alert").textContent).toContain("文件校验失败");
    expect(within(card).getByRole("button", { name: "下载并安装" })).toHaveProperty("disabled", false);
  });
  it("explains protected local modifications without offering to overwrite them", async () => {
    invoke.mockResolvedValue({ ...state(), mods: [{ id: "LiteHub", installed_version: "r1",
      update_available: true, protected: true, message: "本地文件已修改", reason_code: "locally_modified", integrity_checked: true }] });
    render(<ModResourceLibrary edition="CN" en />);
    const card = (await screen.findByText("LiteHub")).closest("article")!;
    expect(within(card).getByText(/Local files have changed/)).toBeTruthy();
    expect(within(card).getByRole("button", { name: "Preserved" })).toHaveProperty("disabled", true);
    expect(within(card).queryByText("本地文件已修改")).toBeNull();
  });
  it("ignores an old edition read that resolves after a new edition", async () => {
    let finish!: (value: ReturnType<typeof state>) => void;
    invoke.mockImplementation((cmd: string, args: { edition?: string }) => cmd === "get_mod_resources" && args.edition === "CN"
      ? new Promise(resolve => { finish = resolve; })
      : Promise.resolve({ ...state(), mods_directory: "G:\\Global\\mods" }));
    const view = render(<ModResourceLibrary edition="CN" en={false} />);
    view.rerender(<ModResourceLibrary edition="Global" en={false} />);
    await screen.findByText("LiteHub");
    await act(async () => finish(state()));
    const card = screen.getByText("LiteHub").closest("article")!;
    await userEvent.click(within(card).getByText("手动下载与安装位置"));
    expect(within(card).getByText("G:\\Global\\mods\\LiteHub")).toBeTruthy();
    expect(screen.queryByText("D:\\Game\\mods\\LiteHub")).toBeNull();
  });
  it("does not install into a stale edition after the native file picker returns", async () => {
    let choose!: (path: string) => void;
    picker.mockImplementation(() => new Promise(resolve => { choose = resolve; }));
    const view = render(<ModResourceLibrary edition="CN" en={false} processorOnly />);
    await userEvent.click(await screen.findByText("手动下载与安装位置"));
    await userEvent.click(screen.getByRole("button", { name: "导入已下载文件" }));
    view.rerender(<ModResourceLibrary edition="Global" en={false} processorOnly />);
    await act(async () => choose("D:\\Downloads\\processor.exe"));
    expect(invoke.mock.calls.some(([command]) => command === "install_mod_resource")).toBe(false);
  });
  it("keeps a completed installation visible when the account catalog refresh fails", async () => {
    const refresh = vi.fn(async () => { throw new Error("账号刷新失败"); });
    render(<ModResourceLibrary edition="CN" en={false} catalog={{ refresh, pool: null } as never} />);
    const card = (await screen.findByText("LiteHub")).closest("article")!;
    await userEvent.click(within(card).getByRole("button", { name: "下载并安装" }));
    expect(await within(card).findByText("已安装到")).toBeTruthy();
    expect(screen.getByRole("alert").textContent).toContain("账号刷新失败");
  });
  it("imports a selected EXE through the same verified installer", async () => {
    picker.mockResolvedValue("D:\\Downloads\\tool.exe");
    const user = userEvent.setup(); render(<ModResourceLibrary edition="CN" en={false} processorOnly />);
    await user.click(await screen.findByText("手动下载与安装位置"));
    await user.click(screen.getByRole("button", { name: "导入已下载文件" }));
    await waitFor(() => expect(invoke).toHaveBeenCalledWith("install_mod_resource", { edition: "CN", resourceId: "processor", localFile: "D:\\Downloads\\tool.exe" }));
  });
});
