import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { useGlobalConfig } from "../../store/globalConfig";
import type { GlobalConfig } from "../../store/types";
import { AccountSelectionMenu } from "./AccountSelectionMenu";

beforeEach(() => useGlobalConfig.setState({ config: null }));
afterEach(cleanup);
const props = {
  selection: { mode: null, ids: [] } as import("../../hooks/useAccountBatch").BatchSelection,
  selectableIds: { launch: ["a", "b"], close: ["c"] },
  onSelectAll: vi.fn(), onClear: vi.fn(),
};

describe("compact account selection menu", () => {
  it("shows a single entry and reveals scopes on demand, with keyboard navigation and focus restoration", async () => {
    const user = userEvent.setup();
    render(<AccountSelectionMenu {...props} />);
    const trigger = screen.getByRole("button", { name: "选择账号" });
    expect(screen.queryByText("全选可启动")).toBeNull();
    trigger.focus();
    await user.keyboard("{ArrowDown}");
    expect(document.activeElement).toBe(screen.getByRole("menuitem", { name: /全选可启动/ }));
    await user.keyboard("{ArrowDown}{Enter}");
    expect(props.onSelectAll).toHaveBeenCalledWith("close");
    expect(screen.queryByRole("menu")).toBeNull();
    expect(document.activeElement).toBe(trigger);
  });

  it("can replace an existing scope, but Escape dismisses the menu without clearing selection", async () => {
    const user = userEvent.setup();
    render(<AccountSelectionMenu {...props} selection={{ mode: "launch", ids: ["a"] }} />);
    const trigger = screen.getByRole("button", { name: "已选 1 个" });
    await user.click(trigger);
    expect(screen.getByRole("menuitem", { name: /全选运行中/ })).toHaveProperty("disabled", false);
    await user.keyboard("{Escape}");
    expect(screen.queryByRole("menu")).toBeNull();
    expect(props.onClear).not.toHaveBeenCalled();
    expect(document.activeElement).toBe(trigger);
    await user.click(screen.getByRole("button", { name: "取消选择" }));
    expect(props.onClear).toHaveBeenCalledOnce();
  });

  it("closes on outside interaction and when execution starts; uncertainty still allows cancellation", () => {
    const { rerender } = render(<AccountSelectionMenu {...props} selection={{ mode: "close", ids: ["c"] }} />);
    fireEvent.click(screen.getByRole("button", { name: "已选 1 个" }));
    fireEvent.pointerDown(document.body);
    expect(screen.queryByRole("menu")).toBeNull();
    fireEvent.click(screen.getByRole("button", { name: "已选 1 个" }));
    rerender(<AccountSelectionMenu {...props} disabled selection={{ mode: "close", ids: ["c"] }} />);
    expect(screen.queryByRole("menu")).toBeNull();
    expect(screen.getByRole("button", { name: "取消选择" })).toHaveProperty("disabled", true);
    rerender(<AccountSelectionMenu {...props} uncertain selection={{ mode: "close", ids: ["c"] }} />);
    expect(screen.getByRole("button", { name: "取消选择" })).toHaveProperty("disabled", false);
  });

  it("skips empty scopes during keyboard navigation and translates without the DOM observer", async () => {
    useGlobalConfig.setState({ config: { app_language: "en-US" } as GlobalConfig });
    const user = userEvent.setup();
    render(<AccountSelectionMenu {...props} selectableIds={{ launch: [], close: ["c"] }} />);
    await user.click(screen.getByRole("button", { name: "Select accounts" }));
    expect(screen.getByRole("menuitem", { name: /Select all ready/ })).toHaveProperty("disabled", true);
    expect(document.activeElement).toBe(screen.getByRole("menuitem", { name: /Select all running/ }));
    await user.keyboard("{Enter}");
    expect(props.onSelectAll).toHaveBeenCalledWith("close");
  });
});
