import { beforeEach, describe, expect, it, vi } from "vitest";
import { useLaunch } from "./launch";
import type { LaunchResult } from "./types";

const mocks = vi.hoisted(() => ({
  invokeCommand: vi.fn(),
  emitEvent: vi.fn(),
  showToast: vi.fn(),
}));
vi.mock("../platform/tauri", () => ({
  invokeCommand: mocks.invokeCommand,
  emitEvent: mocks.emitEvent,
}));
vi.mock("../components/ui/Toast", () => ({ showToast: mocks.showToast }));

const confirmed: LaunchResult = {
  account_id: "one", success: true,
  d2r_pid: 123, error: null, mutex_killed: true,
};
const failed: LaunchResult = {
  ...confirmed, account_id: "two", success: false, d2r_pid: null,
  mutex_killed: false, error: "游戏进程已退出",
};

beforeEach(() => {
  vi.clearAllMocks();
  useLaunch.setState({ launching: false, results: [], error: null, progress: {} });
});

for (const mode of ["accounts", "scheme"] as const) {
  describe(`${mode} launch outcome`, () => {
    const start = () => mode === "accounts"
      ? useLaunch.getState().startLaunch(["one", "two"])
      : useLaunch.getState().startSchemeLaunch([]);

    it("reports a failed login as failure even with a surviving game process", async () => {
      const timeout = { ...confirmed, success: false, error: "登录等待超时" };
      mocks.invokeCommand.mockResolvedValue([confirmed, timeout]);
      await start();
      expect(useLaunch.getState().launching).toBe(false);
      expect(mocks.emitEvent).toHaveBeenCalledWith("launch-ended", { success: false });
      expect(mocks.showToast).not.toHaveBeenCalled();
    });

    it("reports a process failure", async () => {
      mocks.invokeCommand.mockResolvedValue([failed]);
      await start();
      expect(mocks.emitEvent).toHaveBeenCalledWith("launch-ended", { success: false });
    });

    it("still reports confirmed launches as successful", async () => {
      mocks.invokeCommand.mockResolvedValue([confirmed]);
      await start();
      expect(mocks.emitEvent).toHaveBeenCalledWith("launch-ended", {
        success: true,
      });
      expect(mocks.showToast).not.toHaveBeenCalled();
    });
  });
}
