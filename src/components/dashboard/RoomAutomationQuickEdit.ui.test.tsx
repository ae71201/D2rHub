import { readText, writeText } from "@tauri-apps/plugin-clipboard-manager";
import { cleanup, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, beforeEach, expect, it, vi } from "vitest";
import { RoomAutomationQuickEdit } from "./RoomAutomationQuickEdit";
const mocks = vi.hoisted(() => ({ save: vi.fn(), config: { enabled: true, name_prefix: "run-", password: "", next_sequence: 1, sequence_width: 3, primary_account_id: "keep-primary" } }));
vi.mock("../../features/roomAutomation/gateway", () => ({ roomAutomationGateway: {
  startSync: async (handlers: {onConfig: (s: unknown) => void}) => { handlers.onConfig({ generation: 4, config: mocks.config }); return () => {}; },
  getConfig: async () => ({ generation: 5, config: mocks.config }), saveConfig: mocks.save,
} }));
vi.mock("@tauri-apps/plugin-clipboard-manager", () => ({ readText: vi.fn(), writeText: vi.fn(async () => {}) }));
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


it("copies and pastes only the room rule without changing participants", async () => {
  const user = userEvent.setup();
  mocks.save.mockImplementation(async (_generation, config) => ({ snapshot: { generation: 6, config } }));
  vi.mocked(readText).mockResolvedValue(JSON.stringify({ format: "d2rhub-room-rule", version: 1, name_prefix: "friend-", password: "secret", next_sequence: 12, sequence_width: 2 }));
  render(<RoomAutomationQuickEdit active onOpenSettings={() => {}} />);
  await user.click(await screen.findByRole("button", { name: "跟房配置" }));
  await user.click(screen.getByRole("button", { name: "粘贴" }));
  expect((screen.getByLabelText("房名开头") as HTMLInputElement).value).toBe("friend-");
  expect(mocks.save).not.toHaveBeenCalled();
  await user.click(screen.getByRole("button", { name: "复制" }));
  expect(JSON.parse(vi.mocked(writeText).mock.calls[0][0])).toEqual({ format: "d2rhub-room-rule", version: 1, name_prefix: "friend-", password: "secret", next_sequence: 12, sequence_width: 2 });
  expect(screen.queryByLabelText("仅跟房（不创建）")).toBeNull();
  await user.click(screen.getByRole("button", { name: "应用" }));
  expect(mocks.save).toHaveBeenCalledWith(5, expect.objectContaining({ primary_account_id: "keep-primary", name_prefix: "friend-", next_sequence: 12 }), true);
});
it("rejects malformed clipboard rules without changing the current draft", async () => {
  const user = userEvent.setup();
  vi.mocked(readText).mockResolvedValue("bad-json");
  render(<RoomAutomationQuickEdit active onOpenSettings={() => {}} />);
  await user.click(await screen.findByRole("button", { name: "跟房配置" }));
  await user.click(screen.getByRole("button", { name: "粘贴" }));
  expect(await screen.findByRole("alert")).toBeTruthy();
  expect((screen.getByLabelText("房名开头") as HTMLInputElement).value).toBe("run-");
  expect(mocks.save).not.toHaveBeenCalled();
});

it("resets identical pasted rules only after apply, retaining the request on failure", async () => {
  const user = userEvent.setup();
  vi.mocked(readText).mockResolvedValue(JSON.stringify({ format: "d2rhub-room-rule", version: 1, ...mocks.config }));
  mocks.save.mockRejectedValueOnce(new Error("disk error")).mockImplementation(async (_generation, config) => ({ snapshot: { generation: 6, config } }));
  render(<RoomAutomationQuickEdit active onOpenSettings={() => {}} />);
  const trigger = await screen.findByRole("button", { name: "跟房配置" });
  await user.click(trigger);
  await user.click(screen.getByRole("button", { name: "粘贴" }));
  expect(mocks.save).not.toHaveBeenCalled();
  await user.click(screen.getByRole("button", { name: "应用" }));
  expect(await screen.findByRole("alert")).toBeTruthy();
  await user.click(screen.getByRole("button", { name: "应用" }));
  await waitFor(() => expect(screen.queryByRole("dialog")).toBeNull());
  expect(mocks.save).toHaveBeenLastCalledWith(5, expect.objectContaining({ name_prefix: "run-" }), true);
  await user.click(trigger);
  await user.click(screen.getByRole("button", { name: "粘贴" }));
  await user.keyboard("{Escape}");
  await user.click(trigger);
  await user.click(screen.getByRole("button", { name: "应用" }));
  expect(mocks.save).toHaveBeenLastCalledWith(5, expect.objectContaining({ name_prefix: "run-" }));
});

it("applies one collaboration mode with the same room rule and participants", async () => {
  const user = userEvent.setup();
  mocks.save.mockImplementation(async (_generation, config) => ({ snapshot: { generation: 6, config } }));
  render(<RoomAutomationQuickEdit active onOpenSettings={() => {}} />);
  await user.click(await screen.findByRole("button", { name: "跟房配置" }));
  await user.click(screen.getByRole("radio", { name: /拼房跟随/ }));
  expect((screen.getByRole("radio", { name: /拼房跟随/ }) as HTMLInputElement).checked).toBe(true);
  expect((screen.getByRole("radio", { name: /拼房主机/ }) as HTMLInputElement).checked).toBe(false);
  expect(mocks.save).not.toHaveBeenCalled();
  await user.click(screen.getByRole("button", { name: "应用" }));
  await waitFor(() => expect(mocks.save).toHaveBeenCalledWith(5, expect.objectContaining({ collaboration_mode: "follower", primary_account_id: "keep-primary", name_prefix: "run-" })));
});
