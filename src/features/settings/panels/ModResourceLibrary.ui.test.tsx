import { cleanup, render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { ModResourceLibrary } from "./ModResourceLibrary";
const invoke = vi.hoisted(() => vi.fn());
const picker = vi.hoisted(() => vi.fn());
vi.mock("../../../platform/tauri", () => ({ invokeCommand: invoke }));
vi.mock("@tauri-apps/plugin-dialog", () => ({ open: picker }));
vi.mock("../../tasks/taskSync", () => ({ subscribeBeforeReadingTasks: vi.fn(async () => () => {}) }));
const state = () => ({
  catalog: { release_url: "https://github.com/gjy991229/D2rHub/releases", assets: [
    { id: "processor", version: "1.4.0-beta.17", size: 3000000, url: "https://example.test/processor", game_data_version: null },
    { id: "LiteHub", version: "r1", size: 24000000, url: "https://example.test/mod", game_data_version: "93854" },
  ] },
  processor: { ready: false, installed_version: "1.3.3", recommended_version: "1.4.0-beta.17", installed_path: "C:\\old\\d2r-audio-mod.exe", install_directory: "C:\\User\\tools", legacy: true },
  mods_directory: "D:\\Game\\mods", game_data_version: "93854", warning: null,
});
beforeEach(() => { invoke.mockReset(); picker.mockReset(); invoke.mockImplementation(async (cmd: string) => cmd === "get_mod_resources" ? state() : { path: "C:\\User\\tools\\processor.exe" }); });
afterEach(cleanup);
describe("Mod resources", () => {
  it("shows a legacy processor version and installs the update in the managed location", async () => {
    const user = userEvent.setup(); render(<ModResourceLibrary edition="Global" en={false} />);
    await screen.findByText(/检测到旧版内置加工器/);
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
  it("imports a selected EXE through the same verified installer", async () => {
    picker.mockResolvedValue("D:\\Downloads\\tool.exe");
    const user = userEvent.setup(); render(<ModResourceLibrary edition="CN" en={false} processorOnly />);
    await user.click(await screen.findByRole("button", { name: "导入已下载文件" }));
    await waitFor(() => expect(invoke).toHaveBeenCalledWith("install_mod_resource", { edition: "CN", resourceId: "processor", localFile: "D:\\Downloads\\tool.exe" }));
  });
});
