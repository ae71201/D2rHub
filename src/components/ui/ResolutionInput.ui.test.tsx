import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { ResolutionInput } from "./ResolutionInput";

afterEach(cleanup);
describe("custom window resolutions", () => {
  it("accepts custom dimensions and commits a normalized value on blur", () => {
    const change = vi.fn();
    render(<ResolutionInput value="1280x720" onChange={change} />);
    fireEvent.change(screen.getByRole("combobox"), { target: { value: "1366 × 768" } });
    expect(change).not.toHaveBeenCalled();
    fireEvent.blur(screen.getByRole("combobox"));
    expect(change).toHaveBeenCalledWith("1366x768");
  });
  it("enforces 800 by 600 while allowing in-progress typing", () => {
    const change = vi.fn();
    render(<ResolutionInput value="1280x720" onChange={change} />);
    fireEvent.change(screen.getByRole("combobox"), { target: { value: "640x480" } });
    fireEvent.blur(screen.getByRole("combobox"));
    expect(change).toHaveBeenCalledWith("800x600");
    fireEvent.change(screen.getByRole("combobox"), { target: { value: "1200x" } });
    fireEvent.blur(screen.getByRole("combobox"));
    expect(change).toHaveBeenCalledTimes(1);
    expect(screen.getByRole("alert")).toBeTruthy();
  });
});
