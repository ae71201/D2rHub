import { cleanup, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, describe, expect, it, vi } from "vitest";
import { writeText } from "@tauri-apps/plugin-clipboard-manager";
import { ExternalRecognitionTarget } from "./ExternalRecognitionTarget";
import { modState } from "../../mods/workflow/testFixtures";
import type { GlobalConfig } from "../../../store/types";
import type { RuneAudioStatus } from "../audioModuleModel";
vi.mock("@tauri-apps/plugin-clipboard-manager", () => ({ writeText: vi.fn(async () => {}) }));
vi.mock("../../../components/ui/Toast", () => ({ showToast: vi.fn() }));
afterEach(cleanup);
const config = { rune_audio_enabled: true, cn_game_path: "D:\\D2R", global_game_path: "", rune_audio_external_target: { edition: "CN", mod_name: "audio" } } as GlobalConfig;
const state = { ...modState, account_id: "", ready: true, launch_arguments: "-mod audio -txt" };
function props() { return { config, state, status: null, pool: null, busy: false, instances: [], instancesError: null,
  onChange: vi.fn(async () => {}), onSelectInstance: vi.fn(async () => {}), onPrepare: vi.fn() }; }
describe("account-free recognition controls", () => {
  it("copies launcher arguments and has no account initialization step", async () => {
    render(<ExternalRecognitionTarget {...props()} />);
    expect(screen.getByText("已启用，等待游戏运行")).toBeTruthy();
    expect(screen.queryByText("初始化账号")).toBeNull();
    await userEvent.click(screen.getByRole("button", { name: "复制" }));
    expect(writeText).toHaveBeenCalledWith("-mod audio -txt");
  });
  it("shows capture connection separately from verified recognition", () => {
    const status = { running: true, account_id: "", source_id: "external:CN:d:\\d2r", target_pid: 10, decoded_packets: 0 } as RuneAudioStatus;
    const instances = [{ pid: 10, started_at: 1, mod_name: "audio", ready: true, selected: true, message: "" }];
    const view = render(<ExternalRecognitionTarget {...props()} status={status} instances={instances} />);
    expect(screen.getByText("已连接，等待识别声纹")).toBeTruthy();
    view.rerender(<ExternalRecognitionTarget {...props()} status={{ ...status, decoded_packets: 1 }} instances={instances} />);
    expect(screen.getByText("正在识别游戏声音")).toBeTruthy();
  });
  it("selects an explicit process identity when multiple games run", async () => {
    const options = props();
    const instances = [{ pid: 10, started_at: 1, mod_name: "audio", ready: true, message: "" }, { pid: 11, started_at: 2, mod_name: "audio", ready: true, message: "" }];
    render(<ExternalRecognitionTarget {...options} instances={instances} />);
    expect(options.onSelectInstance).not.toHaveBeenCalled();
    await userEvent.selectOptions(screen.getByRole("combobox", { name: "游戏进程" }), "11:2");
    expect(options.onSelectInstance).toHaveBeenCalledWith(instances[1]);
  });
  it("requires manual selection for a single game and keeps its selector visible after connecting", async () => {
    const options = props();
    const instance = { pid: 10, started_at: 1, mod_name: "audio", window_title: "山鬼", ready: true, message: "" };
    const view = render(<ExternalRecognitionTarget {...options} instances={[instance]} />);
    expect(screen.getByText("请选择要监听的游戏进程")).toBeTruthy();
    expect((screen.getByRole("combobox", { name: "游戏进程" }) as HTMLSelectElement).value).toBe("");
    expect(screen.getByRole("option", { name: "山鬼 · PID 10 · audio" })).toBeTruthy();
    expect(options.onSelectInstance).not.toHaveBeenCalled();
    await userEvent.selectOptions(screen.getByRole("combobox", { name: "游戏进程" }), "10:1");
    expect(options.onSelectInstance).toHaveBeenCalledWith(instance);
    const status = { running: true, account_id: "", source_id: "external:CN:d:\\d2r", target_pid: 10, decoded_packets: 0 } as RuneAudioStatus;
    view.rerender(<ExternalRecognitionTarget {...options} status={status} instances={[{ ...instance, selected: true }]} />);
    expect((screen.getByRole("combobox", { name: "游戏进程" }) as HTMLSelectElement).value).toBe("10:1");
    expect(screen.getByText("已连接，等待识别声纹")).toBeTruthy();
    view.rerender(<ExternalRecognitionTarget {...options} status={null} instances={[{ ...instance, started_at: 2 }]} />);
    expect((screen.getByRole("combobox", { name: "游戏进程" }) as HTMLSelectElement).value).toBe("");
    expect(options.onSelectInstance).toHaveBeenCalledTimes(1);
  });
});
