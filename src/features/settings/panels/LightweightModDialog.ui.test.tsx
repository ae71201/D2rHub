import { cleanup, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, beforeEach, expect, it, vi } from "vitest";
import { LightweightModDialog } from "./LightweightModDialog";
import type { ModCapsuleController } from "../../modCapsules/useModCapsulePool";
const mocks = vi.hoisted(() => ({ invoke: vi.fn(), cancel: vi.fn() }));
vi.mock("../../../platform/tauri", () => ({ invokeCommand: mocks.invoke }));
vi.mock("../../tasks/gateway", () => ({ taskGateway: { list: async () => [], subscribe: async () => () => {}, cancel: mocks.cancel } }));
const pool = { generation: 1, scanned_at: "", capsules: [], accounts: [] };
const refresh = vi.fn(async () => pool);
function show(overrides: Partial<ModCapsuleController> = {}) {
  return render(<LightweightModDialog open onClose={vi.fn()} edition="Global" isEnglish={false}
    catalog={{ pool, refresh, loading: false, ...overrides } as unknown as ModCapsuleController} accounts={[]}
    onProcess={vi.fn()} onGenerated={vi.fn()} />);
}
beforeEach(() => {
  vi.clearAllMocks();
  mocks.invoke.mockImplementation(async (command, args) => command === "get_lightweight_mod_context"
    ? { available: true, game_directory: "C:/Game", edition: "Global" }
    : { edition: args.edition, profile: args.profile, mod_name: args.modName, task_id: 1 });
});
afterEach(cleanup);
it("generates a selected profile without any account and does not assign automatically", async () => {
  const user = userEvent.setup(); show();
  await user.click(screen.getByRole("radio", { name: /NullHub/ }));
  expect((screen.getByRole("textbox") as HTMLInputElement).value).toBe("NullHub");
  expect(screen.getByText("此方案的光标和地图显示较小。")).toBeTruthy();
  await user.click(screen.getByRole("button", { name: "开始生成" }));
  await screen.findByText("NullHub 已就绪");
  expect(mocks.invoke).toHaveBeenCalledWith("generate_lightweight_mod", { edition: "Global", profile: "min", modName: "NullHub" });
  expect(mocks.cancel).not.toHaveBeenCalled();
});
it("blocks missing game resources and invalid names", async () => {
  mocks.invoke.mockResolvedValue({ available: false, reason: "请配置游戏目录" });
  const user = userEvent.setup(); show();
  await screen.findByText("请配置游戏目录");
  await user.clear(screen.getByRole("textbox"));
  await user.type(screen.getByRole("textbox"), "../bad");
  await waitFor(() => expect((screen.getByRole("button", { name: "开始生成" }) as HTMLButtonElement).disabled).toBe(true));
  expect(mocks.invoke).not.toHaveBeenCalledWith("generate_lightweight_mod", expect.anything());
});
it("reuses an existing matching profile without invoking generation", async () => {
  const user = userEvent.setup();
  show({ pool: { ...pool, capsules: [{ id: "existing", edition: "Global", name: "LiteHub", origin: "scanned", ready: true, lightweight_profile: "main", launch_arguments: "-mod LiteHub -txt" } as never] } });
  await user.click(await screen.findByRole("button", { name: "使用现有成品" }));
  expect(screen.getByText("LiteHub 已就绪")).toBeTruthy();
  expect(mocks.invoke.mock.calls.every(([command]) => command !== "generate_lightweight_mod")).toBe(true);
});
it("blocks a conflicting name and suggests a separate output", async () => {
  const user = userEvent.setup();
  show({ pool: { ...pool, capsules: [{ edition: "Global", name: "LiteHub", origin: "scanned", ready: true } as never] } });
  expect((screen.getByRole("button", { name: "开始生成" }) as HTMLButtonElement).disabled).toBe(true);
  await user.click(screen.getByRole("button", { name: "另存为 LiteHub-2" }));
  expect((screen.getByRole("textbox") as HTMLInputElement).value).toBe("LiteHub-2");
});
