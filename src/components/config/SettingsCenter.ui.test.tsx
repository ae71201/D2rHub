import type { ReactNode } from "react";
import { act, cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, expect, it, vi } from "vitest";
import { useGlobalConfig } from "../../store/globalConfig";
import { useAccounts } from "../../store/accounts";
import type { GlobalConfig } from "../../store/types";
import type { GlobalConfigPatch } from "../../utils/globalConfigPatch";
import type { SettingsTabId } from "../../features/settings/settingsRegistry";
import { SettingsCenter } from "./SettingsCenter";

const navigation = vi.hoisted(() => ({ result: null as Promise<boolean> | null }));

vi.mock("../../platform/tauri", () => ({
  invokeCommand: vi.fn(async () => null),
  listenEvent: vi.fn(async () => () => {}),
  emitEvent: vi.fn(async () => {}),
}));
vi.mock("@tauri-apps/plugin-dialog", () => ({ open: vi.fn() }));
vi.mock("../../components/ui/Toast", () => ({ showToast: vi.fn() }));
vi.mock("../../features/settings/SettingsShell", () => ({
  SettingsShell: ({ children, activeTab, onTabChange, onClose }: {
    children: ReactNode; activeTab: SettingsTabId;
    onTabChange: (tab: SettingsTabId) => Promise<boolean>;
    onClose: () => void;
  }) => <div>
    <output aria-label="Current section">{activeTab}</output>
    <button onClick={() => { navigation.result = onTabChange("agent"); }}>Change section</button>
    <button onClick={onClose}>Close settings</button>
    {children}
  </div>,
}));
vi.mock("../../features/settings/panels/PathsPanel", () => ({
  PathsPanel: ({ config, updateConfig }: {
    config: GlobalConfig; updateConfig: (update: (config: GlobalConfig) => void) => void;
  }) => <input aria-label="Game path" value={config.cn_game_path}
    onChange={event => updateConfig(next => { next.cn_game_path = event.target.value; })} />,
}));
vi.mock("../../features/settings/panels/LaunchStrategyPanel", () => ({ LaunchStrategyPanel: () => <p>Launch settings</p> }));

const initial = {
  cn_game_path: "C:\\Games\\D2R", global_game_path: "", cn_saved_games_path: "",
  global_saved_games_path: "", installed_optional_modules: [], feature_profile: "normal",
  app_language: "zh-CN", rune_audio_target_account: "", launch_groups: [], theme: "light",
  main_opacity: 95, font_scale: "default",
} as unknown as GlobalConfig;

beforeEach(() => {
  navigation.result = null;
  useGlobalConfig.setState({ config: initial });
  useAccounts.setState({ accounts: [] });
});
afterEach(cleanup);

function mount(onClose = vi.fn()) {
  return render(<SettingsCenter open initialTab="paths" onClose={onClose}
    onReconfigure={vi.fn()} onInitializeAccount={vi.fn()} />);
}

it("keeps navigation pending until the isolated draft is committed", async () => {
  let complete!: (config: GlobalConfig) => void;
  const patch = vi.fn((changes: GlobalConfigPatch) => new Promise<GlobalConfig>(resolve => {
    complete = saved => { useGlobalConfig.setState({ config: saved }); resolve(saved); };
    expect(changes).toEqual({ cn_game_path: "D:\\D2R" });
  }));
  useGlobalConfig.setState({ patch });
  mount();
  fireEvent.change(await screen.findByLabelText("Game path"), { target: { value: "D:\\D2R" } });
  expect(useGlobalConfig.getState().config?.cn_game_path).toBe(initial.cn_game_path);
  fireEvent.click(screen.getByText("Change section"));
  expect(navigation.result).toBeInstanceOf(Promise);
  expect(screen.getByLabelText("Current section").textContent).toBe("paths");
  await act(async () => {
    complete({ ...initial, cn_game_path: "D:\\D2R" });
    expect(await navigation.result).toBe(true);
  });
  expect(screen.getByLabelText("Current section").textContent).toBe("agent");
});

it("returns false and preserves the visible draft when navigation save fails", async () => {
  const patch = vi.fn(async () => { throw new Error("disk full"); });
  useGlobalConfig.setState({ patch });
  mount();
  fireEvent.change(await screen.findByLabelText("Game path"), { target: { value: "D:\\D2R" } });
  fireEvent.click(screen.getByText("Change section"));
  await act(async () => { expect(await navigation.result).toBe(false); });
  expect(screen.getByLabelText("Current section").textContent).toBe("paths");
  expect((screen.getByLabelText("Game path") as HTMLInputElement).value).toBe("D:\\D2R");
  expect(useGlobalConfig.getState().config?.cn_game_path).toBe(initial.cn_game_path);
  await waitFor(() => expect(patch).toHaveBeenCalledOnce());
});

it("keeps settings open when the user types another change during close-save", async () => {
  let complete!: (config: GlobalConfig) => void;
  const pending = new Promise<GlobalConfig>(resolve => {
    complete = saved => { useGlobalConfig.setState({ config: saved }); resolve(saved); };
  });
  useGlobalConfig.setState({ patch: vi.fn(() => pending) });
  const onClose = vi.fn();
  mount(onClose);
  fireEvent.change(await screen.findByLabelText("Game path"), { target: { value: "D:\\D2R" } });
  fireEvent.click(screen.getByText("Close settings"));
  fireEvent.change(screen.getByLabelText("Game path"), { target: { value: "E:\\D2R" } });
  await act(async () => {
    complete({ ...initial, cn_game_path: "D:\\D2R" });
    await pending;
  });
  expect(onClose).not.toHaveBeenCalled();
  expect((screen.getByLabelText("Game path") as HTMLInputElement).value).toBe("E:\\D2R");
  expect(useGlobalConfig.getState().config?.cn_game_path).toBe("D:\\D2R");
});
