import { act, cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { HubModSettings, type HubSettings } from "./HubModSettings";

const { invoke } = vi.hoisted(() => ({ invoke: vi.fn() }));
vi.mock("../../../platform/tauri/client", () => ({ invokeCommand: invoke }));
const config = (waypoints = true): HubSettings => ({ base_mod: "LiteHub", game_data_version: "93854", waypoints_supported: waypoints, etag: "before" });
beforeEach(() => { invoke.mockReset(); invoke.mockImplementation(async (command) => command === "get_hub_mod_settings" ? config() : null); });
afterEach(cleanup);

describe("Hub product settings", () => {
  it("supports a renamed derivative and saves the version with its edit token", async () => {
    render(<HubModSettings edition="CN" modName="MyLiteAudio" en={false} disabled={false} />);
    const input = await screen.findByRole("textbox", { name: "游戏数据版本" });
    expect(screen.getByRole("heading", { name: "第四幕快捷传送" })).toBeTruthy();
    expect(invoke).toHaveBeenCalledWith("get_mod_waypoints", { edition: "CN", modName: "MyLiteAudio" });
    fireEvent.change(input, { target: { value: "93855" } });
    invoke.mockResolvedValueOnce({ ...config(), game_data_version: "93855", etag: "after" });
    fireEvent.click(screen.getByRole("button", { name: "保存数据版本" }));
    await screen.findByText("已保存，重新启动游戏后生效。");
    expect(invoke).toHaveBeenLastCalledWith("save_hub_mod_data_version", { edition: "CN", modName: "MyLiteAudio", gameDataVersion: "93855", etag: "before" });
  });
  it("hides unsupported controls and blocks invalid versions", async () => {
    invoke.mockResolvedValue(config(false));
    render(<HubModSettings edition="Global" modName="NullCustom" en={true} disabled={false} />);
    const input = await screen.findByRole("textbox", { name: "Game data version" });
    expect(screen.queryByRole("heading", { name: "Act IV waypoint shortcuts" })).toBeNull();
    for (const value of ["", "0", "abc", "93854.1", "4294967296"]) {
      fireEvent.change(input, { target: { value } });
      expect(screen.getByRole("button", { name: "Save data version" }).hasAttribute("disabled")).toBe(true);
    }
  });
  it("keeps the draft after a running-game or stale-write rejection", async () => {
    render(<HubModSettings edition="CN" modName="LiteHub" en={false} disabled={false} />);
    const input = await screen.findByRole("textbox", { name: "游戏数据版本" });
    fireEvent.change(input, { target: { value: "93855" } });
    invoke.mockRejectedValueOnce(new Error("请先关闭游戏"));
    fireEvent.click(screen.getByRole("button", { name: "保存数据版本" }));
    await screen.findByText(/请先关闭游戏/);
    expect((input as HTMLInputElement).value).toBe("93855");
    expect(screen.queryByText("已保存，重新启动游戏后生效。")).toBeNull();
  });
  it("does not render settings when the backend cannot verify Hub ancestry", async () => {
    invoke.mockResolvedValue(null);
    const view = render(<HubModSettings edition="CN" modName="Other" en={false} disabled={false} />);
    await act(async () => {});
    expect(view.container.textContent).toBe("");
  });
});
