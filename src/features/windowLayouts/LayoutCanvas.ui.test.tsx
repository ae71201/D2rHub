import { act, cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, expect, it, vi } from "vitest";
import type { WindowLayout } from "../../store/types";
import { LayoutCanvas } from "./LayoutCanvas";

afterEach(() => { cleanup(); vi.unstubAllGlobals(); });

it.each(["pointerCancel", "lostPointerCapture"] as const)(
  "keeps the next drag live after %s cancels a pending frame",
  cancellation => {
    class TestPointerEvent extends MouseEvent {
      readonly pointerId: number;
      constructor(type: string, init: PointerEventInit = {}) {
        super(type, init);
        this.pointerId = init.pointerId ?? 1;
      }
    }
    vi.stubGlobal("PointerEvent", TestPointerEvent);
    const frames = new Map<number, FrameRequestCallback>();
    let nextFrame = 0;
    vi.stubGlobal("requestAnimationFrame", (callback: FrameRequestCallback) => {
      frames.set(++nextFrame, callback);
      return nextFrame;
    });
    vi.stubGlobal("cancelAnimationFrame", (id: number) => { frames.delete(id); });
    const rect = { x: 0, y: 0, width: 1920, height: 1080 };
    const layout: WindowLayout = {
      id: "desk", name: "Desk",
      monitors: [{ id: "main", name: "Main", primary: true, scale_factor: 1, bounds: rect, work_area: rect }],
      windows: [{ monitor_id: "main", x: 100, y: 100, width: 800, height: 600 }],
    };
    const onChange = vi.fn();
    render(<LayoutCanvas layout={layout} labels={["Account"]} selected={0} onSelect={() => {}}
      onChange={onChange} english />);
    const canvas = screen.getByRole("group");
    Object.assign(canvas, { setPointerCapture: vi.fn(), hasPointerCapture: () => false });
    const window = screen.getByRole("button");
    fireEvent.pointerDown(window, { button: 0, pointerId: 1, clientX: 100, clientY: 100 });
    fireEvent.pointerMove(canvas, { pointerId: 1, clientX: 120, clientY: 120 });
    expect(frames.size).toBe(1);
    fireEvent[cancellation](canvas, { pointerId: 1 });
    expect(frames.size).toBe(0);
    expect(onChange).toHaveBeenLastCalledWith(layout.windows);
    onChange.mockClear();

    fireEvent.pointerDown(window, { button: 0, pointerId: 2, clientX: 100, clientY: 100 });
    fireEvent.pointerMove(canvas, { pointerId: 2, clientX: 150, clientY: 150 });
    expect(frames.size).toBe(1);
    act(() => {
      const callbacks = [...frames.values()];
      frames.clear();
      callbacks.forEach(callback => callback(16));
    });
    // The new position is published before pointer-up, not just on release.
    expect(onChange).toHaveBeenCalledTimes(1);
    expect(onChange.mock.calls[0][0][0].x).toBeGreaterThan(100);
    fireEvent.pointerUp(canvas, { pointerId: 2 });
  },
);

it("draws the caption at measured physical height and excludes invisible borders", () => {
  const rect = { x: 0, y: 0, width: 2560, height: 1440 };
  const layout: WindowLayout = { id: "frame", name: "Frame", monitors: [{ id: "main", name: "Main",
    primary: true, scale_factor: 1, bounds: rect, work_area: rect, frame: { dpi: 96, style: 0, ex_style: 0,
      visible: { left: 1, top: 31, right: 1, bottom: 1 }, invisible: { left: 7, top: 0, right: 7, bottom: 7 } } }],
    windows: [{ monitor_id: "main", x: 0, y: 0, width: 1280, height: 720 }] };
  const { container } = render(<LayoutCanvas layout={layout} labels={["Account"]} selected={0}
    onSelect={() => {}} onChange={() => {}} />);
  expect(container.querySelector(".layout-window-body")?.getAttribute("width")).toBe("1282");
  expect(container.querySelector(".layout-window-body")?.getAttribute("height")).toBe("752");
  expect(container.querySelector(".layout-window-caption rect")?.getAttribute("height")).toBe("31");
  expect(container.querySelector(".layout-window-client")?.getAttribute("y")).toBe("31");
  expect(container.querySelector(".layout-window-client")?.getAttribute("height")).toBe("720");
});
