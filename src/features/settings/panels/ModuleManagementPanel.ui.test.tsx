import { cleanup, render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, describe, expect, it, vi } from "vitest";
import type { GlobalConfig } from "../../../store/types";
import { ModuleManagementPanel } from "./ModuleManagementPanel";

const config = { app_language: "zh-CN" } as GlobalConfig;
afterEach(cleanup);

describe("ModuleManagementPanel", () => {
  it("offers configuration for added extensions and contextual dependencies for adding recognition", async () => {
    const onInstall = vi.fn();
    const onOpen = vi.fn();
    render(<ModuleManagementPanel config={config} installedModules={["pet"]} onInstall={onInstall} onUninstall={vi.fn()} onOpen={onOpen} />);
    const recognition = screen.getByRole("article", { name: "识别与统计" });
    expect(within(recognition).getByText(/同时添加桌面悬浮窗/)).toBeTruthy();
    expect(screen.queryByRole("note")).toBeNull();
    await userEvent.click(within(recognition).getByRole("button", { name: "添加识别与统计" }));
    expect(onInstall).toHaveBeenCalledWith("automation");
    await userEvent.click(screen.getByRole("button", { name: "设置桌宠" }));
    expect(onOpen).toHaveBeenCalledWith("pet");
  });

  it("explains the dependent removal only when requested and requires confirmation", async () => {
    const onUninstall = vi.fn();
    render(<ModuleManagementPanel config={config} installedModules={["automation", "overlays"]} onInstall={vi.fn()} onUninstall={onUninstall} onOpen={vi.fn()} />);
    expect(screen.queryByRole("alert")).toBeNull();
    await userEvent.click(screen.getByRole("button", { name: "移除桌面悬浮窗" }));
    expect(screen.getByRole("alert").textContent).toContain("会一并移除");
    expect(onUninstall).not.toHaveBeenCalled();
    await userEvent.click(screen.getByRole("button", { name: "确认移除" }));
    expect(onUninstall).toHaveBeenCalledWith("overlays");
  });

  it("cancels removal without modifying any extension", async () => {
    const onUninstall = vi.fn();
    render(<ModuleManagementPanel config={config} installedModules={["pet"]} onInstall={vi.fn()} onUninstall={onUninstall} onOpen={vi.fn()} />);
    await userEvent.click(screen.getByRole("button", { name: "移除桌宠" }));
    await userEvent.click(screen.getByRole("button", { name: "取消" }));
    expect(onUninstall).not.toHaveBeenCalled();
    expect(screen.queryByRole("alert")).toBeNull();
  });
});
