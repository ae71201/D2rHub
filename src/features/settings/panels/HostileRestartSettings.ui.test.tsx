import { fireEvent, render, screen, cleanup, waitFor } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import type { GlobalConfig } from "../../../store/types";
import { HostileRestartSettings } from "./HostileRestartSettings";
vi.mock("../../../platform/tauri", () => ({ listenEvent: vi.fn(async () => () => undefined) }));
afterEach(cleanup);
describe("hostile restart settings", () => {
  it("enables independently without enabling drop/scene recognition", async () => {
    const persist = vi.fn(async () => undefined);
    const config = { app_language: "zh-CN", rune_audio_enabled: false, rune_audio_target_account: "one" } as GlobalConfig;
    render(<HostileRestartSettings config={config} persist={persist} />);
    fireEvent.click(screen.getByRole("switch", { name: "启用敌对自动重开" }));
    await waitFor(() => expect(persist).toHaveBeenCalledWith(expect.objectContaining({
      hostile_auto_restart_enabled: true, rune_audio_enabled: false, rune_audio_target_account: "one",
    })));
  });
  it("defaults to all game windows and changes only execution scope", async () => {
    const persist = vi.fn(async () => undefined);
    const config = { app_language: "zh-CN", rune_audio_target_account: "one", hostile_auto_restart_enabled: true } as GlobalConfig;
    render(<HostileRestartSettings config={config} persist={persist} />);
    const select = screen.getByRole("combobox", { name: "触发范围" }) as HTMLSelectElement;
    expect(select.value).toBe("all");
    fireEvent.change(select, { target: { value: "listening" } });
    await waitFor(() => expect(persist).toHaveBeenCalledWith(expect.objectContaining({
      hostile_auto_restart_enabled: true, hostile_auto_restart_all_windows: false, rune_audio_target_account: "one",
    })));
  });
});

it("keeps the controls visible when collapsed and expands only explanatory content", () => {
  const config = { app_language: "zh-CN", hostile_auto_restart_all_windows: false } as GlobalConfig;
  render(<HostileRestartSettings config={config} persist={vi.fn(async () => {})} />);
  const disclosure = screen.getByRole("button", { name: "敌对自动重开" });
  expect(disclosure.getAttribute("aria-expanded")).toBe("false");
  expect((screen.getByRole("combobox", { name: "触发范围" }) as HTMLSelectElement).value).toBe("listening");
  expect(screen.getByRole("switch", { name: "启用敌对自动重开" })).toBeTruthy();
  const description = screen.getByText(/共用识别页面的监听目标/).closest("[hidden]");
  expect(description).toBeTruthy();
  fireEvent.click(disclosure);
  expect(disclosure.getAttribute("aria-expanded")).toBe("true");
  expect(screen.getByText(/共用识别页面的监听目标/).closest("[hidden]")).toBeNull();
});

it("surfaces save errors even while explanations are collapsed", async () => {
  render(<HostileRestartSettings config={{} as GlobalConfig} persist={vi.fn(async () => { throw new Error("disk failure"); })} />);
  fireEvent.click(screen.getByRole("switch", { name: "启用敌对自动重开" }));
  expect((await screen.findByRole("alert")).textContent).toBe("Error: disk failure");
  expect(screen.getByRole("button", { name: "敌对自动重开" }).getAttribute("aria-expanded")).toBe("false");
});
