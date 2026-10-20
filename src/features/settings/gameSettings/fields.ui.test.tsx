import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, expect, it, vi } from "vitest";
import { GameplaySection } from "./fields";

afterEach(cleanup);
it("preserves boolean and numeric switch representations", () => {
  const update = vi.fn();
  render(<GameplaySection settings={{ "Camera Shake": true, "Always Run": 0 }} update={update} />);
  fireEvent.click(screen.getByRole("switch", { name: "镜头震动" }));
  fireEvent.click(screen.getByRole("switch", { name: "始终奔跑" }));
  expect(update.mock.calls).toEqual([["Camera Shake", false], ["Always Run", 1]]);
});
it("does not invent missing switches or interpret unexpected enum values", () => {
  render(<GameplaySection settings={{ "Camera Shake": 2 }} update={vi.fn()} />);
  expect(screen.queryByRole("switch", { name: "镜头震动" })).toBeNull();
  expect(screen.queryByRole("switch", { name: "始终奔跑" })).toBeNull();
});
