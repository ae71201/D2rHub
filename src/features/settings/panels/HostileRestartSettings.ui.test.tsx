import { fireEvent, render, screen, cleanup } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import type { GlobalConfig } from "../../../store/types";
import { HostileRestartSettings } from "./HostileRestartSettings";
vi.mock("../../../platform/tauri", () => ({ listenEvent: vi.fn(async () => () => undefined) }));
afterEach(cleanup);
describe("hostile restart settings", () => {
  it("enables independently without enabling drop/scene recognition", () => {
    const persist = vi.fn(async () => undefined);
    const config = { app_language: "zh-CN", rune_audio_enabled: false, rune_audio_target_account: "one" } as GlobalConfig;
    render(<HostileRestartSettings config={config} persist={persist} />);
    fireEvent.click(screen.getByRole("switch", { name: "启用敌对自动重开" }));
    expect(persist).toHaveBeenCalledWith(expect.objectContaining({
      hostile_auto_restart_enabled: true, rune_audio_enabled: false, rune_audio_target_account: "one",
    }));
  });
  it("defaults to the monitored window and changes only execution scope", () => {
    const persist = vi.fn(async () => undefined);
    const config = { app_language: "zh-CN", rune_audio_target_account: "one", hostile_auto_restart_enabled: true } as GlobalConfig;
    render(<HostileRestartSettings config={config} persist={persist} />);
    const select = screen.getByRole("combobox", { name: "触发范围" }) as HTMLSelectElement;
    expect(select.value).toBe("listening");
    fireEvent.change(select, { target: { value: "all" } });
    expect(persist).toHaveBeenCalledWith(expect.objectContaining({
      hostile_auto_restart_enabled: true, hostile_auto_restart_all_windows: true, rune_audio_target_account: "one",
    }));
  });
});
