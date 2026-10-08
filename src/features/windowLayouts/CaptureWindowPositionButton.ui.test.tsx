import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { useAccounts } from "../../store/accounts";
import type { AccountMeta } from "../../store/types";
import { CaptureWindowPositionButton } from "./CaptureWindowPositionButton";

const mocks = vi.hoisted(() => ({ invoke: vi.fn(), toast: vi.fn() }));
vi.mock("../../platform/tauri", () => ({ invokeCommand: mocks.invoke }));
vi.mock("../../components/ui/Toast", () => ({ showToast: mocks.toast }));
const saved = { id: "acount1", window_x: -1600, window_y: 120, active_position_id: "captured", position_presets: [{ id: "captured", name: "当前位置", x: -1600, y: 120 }] } as AccountMeta;
beforeEach(() => {
  useAccounts.setState({ accounts: [{ id: "acount1", is_running: true, display_name: "First" } as AccountMeta] });
  mocks.invoke.mockResolvedValue(saved);
});
afterEach(cleanup);
describe("current account window capture", () => {
  it("captures through the account command and keeps live runtime state intact", async () => {
    render(<CaptureWindowPositionButton accountId="acount1" />);
    fireEvent.click(screen.getByRole("button", { name: "存储当前位置" }));
    await waitFor(() => expect(useAccounts.getState().accounts[0].active_position_id).toBe("captured"));
    expect(mocks.invoke).toHaveBeenCalledWith("capture_account_window_position", { accountId: "acount1", activate: true });
    expect(useAccounts.getState().accounts[0].is_running).toBe(true);
    expect(useAccounts.getState().accounts[0].window_x).toBe(-1600);
  });
  it("keeps default position selection when capturing a launch-scheme preset", async () => {
    const captured = vi.fn();
    render(<CaptureWindowPositionButton accountId="acount1" activate={false} onCaptured={captured} />);
    fireEvent.click(screen.getByRole("button", { name: "存储当前位置" }));
    await waitFor(() => expect(captured).toHaveBeenCalledWith(saved));
    expect(mocks.invoke).toHaveBeenCalledWith("capture_account_window_position", { accountId: "acount1", activate: false });
  });
  it("does not capture after unsaved account settings fail to save", async () => {
    render(<CaptureWindowPositionButton accountId="acount1" beforeCapture={async () => false} />);
    fireEvent.click(screen.getByRole("button", { name: "存储当前位置" }));
    await waitFor(() => expect((screen.getByRole("button", { name: "存储当前位置" }) as HTMLButtonElement).disabled).toBe(false));
    expect(mocks.invoke).not.toHaveBeenCalled();
  });
});
