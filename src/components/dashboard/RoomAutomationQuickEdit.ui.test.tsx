import { cleanup, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, beforeEach, expect, it, vi } from "vitest";
import { RoomAutomationQuickEdit } from "./RoomAutomationQuickEdit";
const mocks = vi.hoisted(() => ({ save: vi.fn(), config: { enabled: true, name_prefix: "run-", password: "", next_sequence: 1, sequence_width: 3, primary_account_id: "keep-primary" } }));
vi.mock("../../features/roomAutomation/gateway", () => ({ roomAutomationGateway: {
  startSync: async (handlers: {onConfig: (s: unknown) => void}) => { handlers.onConfig({ generation: 4, config: mocks.config }); return () => {}; },
  getConfig: async () => ({ generation: 5, config: mocks.config }), saveConfig: mocks.save,
} }));
vi.mock("../ui/Toast", () => ({ showToast: vi.fn() }));
afterEach(cleanup);
beforeEach(() => { vi.clearAllMocks(); });
it("cancels with Escape without saving and returns focus to its trigger", async () => {
  const user = userEvent.setup();
  render(<RoomAutomationQuickEdit active onOpenSettings={() => {}} />);
  const trigger = await screen.findByRole("button", { name: "跟房配置" });
  await user.click(trigger);
  await user.clear(screen.getByLabelText("房名开头"));
  await user.type(screen.getByLabelText("房名开头"), "changed-");
  await user.keyboard("{Escape}");
  expect(screen.queryByRole("dialog")).toBeNull();
  expect(mocks.save).not.toHaveBeenCalled();
  expect(document.activeElement).toBe(trigger);
  await user.click(trigger);
  expect((screen.getByLabelText("房名开头") as HTMLInputElement).value).toBe("run-");
});
it("preserves the draft on save failure and saves against the latest generation on retry", async () => {
  const user = userEvent.setup();
  mocks.save.mockRejectedValueOnce(new Error("disk error")).mockResolvedValueOnce({ snapshot: { generation: 6, config: mocks.config } });
  render(<RoomAutomationQuickEdit active onOpenSettings={() => {}} />);
  await user.click(await screen.findByRole("button", { name: "跟房配置" }));
  await user.clear(screen.getByLabelText("房名开头"));
  await user.type(screen.getByLabelText("房名开头"), "new-");
  await user.click(screen.getByRole("button", { name: "应用" }));
  expect(await screen.findByRole("alert")).toBeTruthy();
  expect((screen.getByLabelText("房名开头") as HTMLInputElement).value).toBe("new-");
  await user.click(screen.getByRole("button", { name: "应用" }));
  await waitFor(() => expect(screen.queryByRole("dialog")).toBeNull());
  expect(mocks.save).toHaveBeenLastCalledWith(5, expect.objectContaining({ name_prefix: "new-", primary_account_id: "keep-primary" }));
});

