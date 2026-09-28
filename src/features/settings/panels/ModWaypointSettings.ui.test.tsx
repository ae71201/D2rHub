import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { ModWaypointSettings, type WaypointConfig } from "./ModWaypointSettings";

const { invoke } = vi.hoisted(() => ({ invoke: vi.fn() }));
vi.mock("../../../platform/tauri/client", () => ({ invokeCommand: invoke }));
const config = (): WaypointConfig => ({
  selected: ["a", "b", "c", "d", "e", "f"], defaults: ["b", "a", "c", "d", "e", "f"], etag: "original",
  options: "abcdefg".split("").map((id, index) => ({ id, act: index < 3 ? 1 : 3, label_zh: `地点${id}`, label_en: `Place ${id}` })),
});
beforeEach(() => { invoke.mockReset(); invoke.mockResolvedValue(config()); });
afterEach(cleanup);

describe("Act IV waypoint editor", () => {
  it("saves ordered slots and etag; defaults do not write until saved", async () => {
    render(<ModWaypointSettings edition="CN" modName="LiteHub" en={false} disabled={false} />);
    await screen.findByLabelText("第 4 项");
    fireEvent.click(screen.getByRole("button", { name: "恢复默认顺序" }));
    expect(invoke).toHaveBeenCalledTimes(1);
    invoke.mockResolvedValueOnce({ ...config(), selected: config().defaults, etag: "updated" });
    fireEvent.click(screen.getByRole("button", { name: "保存传送列表" }));
    await screen.findByText("已保存，重新启动游戏后生效。");
    expect(invoke).toHaveBeenLastCalledWith("save_mod_waypoints", {
      edition: "CN", modName: "LiteHub", selected: config().defaults, etag: "original",
    });
  });
  it("blocks duplicates but allows empty slots", async () => {
    render(<ModWaypointSettings edition="CN" modName="BoHub" en={false} disabled={false} />);
    const slot = await screen.findByLabelText("第 4 项");
    fireEvent.change(slot, { target: { value: "b" } });
    expect(screen.getByRole("button", { name: "保存传送列表" }).hasAttribute("disabled")).toBe(true);
    fireEvent.change(slot, { target: { value: "" } });
    expect(screen.getByRole("button", { name: "保存传送列表" }).hasAttribute("disabled")).toBe(false);
  });
  it("keeps draft and reports game-in-use failure without claiming success", async () => {
    render(<ModWaypointSettings edition="CN" modName="LiteHub" en={false} disabled={false} />);
    fireEvent.change(await screen.findByLabelText("第 4 项"), { target: { value: "g" } });
    invoke.mockRejectedValueOnce(new Error("请先关闭游戏，再保存传送列表"));
    fireEvent.click(screen.getByRole("button", { name: "保存传送列表" }));
    await waitFor(() => expect(screen.getByRole("alert").textContent).toContain("请先关闭游戏"));
    expect((screen.getByLabelText("第 4 项") as HTMLSelectElement).value).toBe("g");
    expect(screen.queryByText("已保存，重新启动游戏后生效。")).toBeNull();
  });
});
