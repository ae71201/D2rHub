import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { createPortal } from "react-dom";

import type { AccountMeta } from "../../store/types";
import { SortableAccountCard } from "./SortableAccountCard";

const mocks = vi.hoisted(() => ({ dragKeyDown: vi.fn(), dragMouseDown: vi.fn(), dragTouchStart: vi.fn() }));

vi.mock("@dnd-kit/sortable", () => ({
  useSortable: () => ({
    attributes: { tabIndex: 0 },
    listeners: { onKeyDown: mocks.dragKeyDown, onMouseDown: mocks.dragMouseDown, onTouchStart: mocks.dragTouchStart },
    setNodeRef: vi.fn(),
    setActivatorNodeRef: vi.fn(),
    transform: null,
    transition: undefined,
    isDragging: false,
  }),
}));

vi.mock("@dnd-kit/utilities", () => ({
  CSS: { Transform: { toString: () => undefined } },
}));

vi.mock("../../store/launch", () => ({
  useLaunch: () => ({ progress: {} }),
}));

vi.mock("./AccountCard", () => ({
  AccountGridItem: () => <>{createPortal(<p>弹层空白区域</p>, document.body)}<p>卡片空白区域</p><input aria-label="Mod 参数" /><button><span>启动</span></button><label><input type="checkbox" />选中</label><select aria-label="Mod"><option>原版</option></select></>,
}));

const account: AccountMeta = {
  id: "account-1",
  display_name: "一号账号",
  mod_args: "",
  created_at: "2026-08-31T00:00:00Z",
  last_launched_at: null,
  last_reset_at: null,
  initialized: true,
  order: 0,
  is_running: false,
};

afterEach(cleanup);
beforeEach(() => vi.clearAllMocks());

describe("SortableAccountCard drag boundary", () => {
  it("keeps spaces in the Mod input without activating keyboard drag", async () => {
    const user = userEvent.setup();
    const { container } = render(
      <SortableAccountCard
        account={account}
        onRename={vi.fn()}
        onDelete={vi.fn()}
        onConfigure={vi.fn()}
        onLaunch={vi.fn()}
        onBattleNetOnly={vi.fn()}
      />,
    );

    const input = screen.getByRole("textbox", { name: "Mod 参数" });
    await user.type(input, "-mod alpha beta");

    expect(input).toHaveProperty("value", "-mod alpha beta");
    expect(mocks.dragKeyDown).not.toHaveBeenCalled();

    const sortable = container.querySelector("[data-sortable-account-id]");
    expect(sortable).not.toBeNull();
    fireEvent.keyDown(sortable!, { key: " " });
    expect(mocks.dragKeyDown).toHaveBeenCalledOnce();
    fireEvent.mouseDown(screen.getByText("卡片空白区域"));
    fireEvent.touchStart(screen.getByText("卡片空白区域"));
    expect(mocks.dragMouseDown).toHaveBeenCalledOnce();
    expect(mocks.dragTouchStart).toHaveBeenCalledOnce();
    for (const target of [screen.getByText("弹层空白区域"), input, screen.getByText("启动"), screen.getByText("选中"), screen.getByLabelText("Mod")]) {
      fireEvent.mouseDown(target); fireEvent.touchStart(target); fireEvent.keyDown(target, { key: " " });
    }
    expect(mocks.dragMouseDown).toHaveBeenCalledOnce();
    expect(mocks.dragTouchStart).toHaveBeenCalledOnce();
    expect(mocks.dragKeyDown).toHaveBeenCalledOnce();
    expect(screen.queryByRole("button", { name: /拖动排序/ })).toBeNull();
  });
});

it("disables dragging during account selection", () => {
  const view = render(<SortableAccountCard account={account} isSelectionMode onRename={vi.fn()} onDelete={vi.fn()} onConfigure={vi.fn()} onLaunch={vi.fn()} onBattleNetOnly={vi.fn()} />);
  const root = view.container.querySelector("[data-sortable-account-id]")!;
  fireEvent.keyDown(root, { key: " " });
  fireEvent.mouseDown(screen.getByText("卡片空白区域"));
  fireEvent.touchStart(screen.getByText("卡片空白区域"));
  expect(root.getAttribute("tabindex")).toBe("-1");
  expect(mocks.dragKeyDown).not.toHaveBeenCalled();
  expect(mocks.dragMouseDown).not.toHaveBeenCalled();
  expect(mocks.dragTouchStart).not.toHaveBeenCalled();
});
