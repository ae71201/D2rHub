import { afterEach, beforeEach, expect, it, vi } from "vitest";
import { cleanup, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { RecognitionQuickEdit } from "./RecognitionQuickEdit";
import { useAccounts } from "../../store/accounts";
import { useGlobalConfig } from "../../store/globalConfig";
import type { AccountMeta, GlobalConfig } from "../../store/types";
const patch = vi.fn();
afterEach(cleanup);
beforeEach(() => {
  patch.mockReset().mockResolvedValue(undefined);
  useGlobalConfig.setState({ config: { app_language: "zh-CN", rune_audio_target_account: "1" } as GlobalConfig, patch, saving: false });
  useAccounts.setState({ accounts: [{ id: "1", display_name: "主号一", initialized: true }, { id: "2", display_name: "主号二", initialized: true }] as AccountMeta[] });
});
it("changes only recognition source", async () => {
  const user = userEvent.setup(); render(<RecognitionQuickEdit />);
  await user.click(screen.getByRole("button", { name: "识别配置" }));
  await user.click(screen.getByLabelText("主号二"));
  expect(patch).toHaveBeenCalledWith({ rune_audio_target_account: "2", rune_audio_external_target: null });
});
it("shows save failure and keeps the previous selection", async () => {
  patch.mockRejectedValue(new Error("保存失败"));
  const user = userEvent.setup(); render(<RecognitionQuickEdit />);
  await user.click(screen.getByRole("button", { name: "识别配置" }));
  await user.click(screen.getByLabelText("主号二"));
  await waitFor(() => expect(screen.getByRole("alert").textContent).toContain("保存失败"));
  expect((screen.getByLabelText("主号一") as HTMLInputElement).checked).toBe(true);
});
