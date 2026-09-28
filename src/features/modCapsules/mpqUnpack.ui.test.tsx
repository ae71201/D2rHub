import { act, cleanup, render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { invokeCommand, listenEvent } from "../../platform/tauri";
import type { ModCapsulePool, ModUnpackProgress, ModUnpackResult } from "../../store/types";
import { ModCatalogManager } from "../settings/panels/ModCatalogManager";
import { useModCapsulePool } from "./useModCapsulePool";

vi.mock("../../platform/tauri", () => ({ invokeCommand: vi.fn(), listenEvent: vi.fn() }));
vi.mock("../../components/ui/Toast", () => ({ showToast: vi.fn() }));

const packed: ModCapsulePool = {
  generation: 1, scanned_at: "before", accounts: [], capsules: [{
    id: "scan:cn:mini", name: "mini", edition: "CN", origin: "scanned",
    launch_arguments: "-mod mini -txt", default_launch_arguments: "-mod mini -txt",
    feature_groups: [], processed: false, source_eligible: false, requires_unpack: true,
    update_required: false, ready: true, deletable: true, assigned_account_ids: [],
  }],
};
const unpacked: ModCapsulePool = { ...packed, scanned_at: "after", capsules: [{ ...packed.capsules[0], requires_unpack: false, source_eligible: true }] };
let progressHandler: ((event: { payload: ModUnpackProgress }) => void) | undefined;
const unlisten = vi.fn();
function Harness({ onProcess }: { onProcess: () => void }) {
  const catalog = useModCapsulePool({ active: true });
  return <ModCatalogManager catalog={catalog} accounts={[]} onProcess={onProcess} />;
}
beforeEach(() => {
  vi.resetAllMocks(); progressHandler = undefined;
  vi.mocked(listenEvent).mockImplementation(async (_name, handler) => { progressHandler = handler as typeof progressHandler; return unlisten; });
  vi.mocked(invokeCommand).mockImplementation(async command => {
    if (command === "get_mod_capsule_pool") return packed as never;
    return undefined as never;
  });
});
afterEach(cleanup);

describe("MPQ preset unpack workflow", () => {
  it("unpacks separately, then changes the same preset action to processing", async () => {
    let finish!: (result: ModUnpackResult) => void;
    const pending = new Promise<ModUnpackResult>(resolve => { finish = resolve; });
    vi.mocked(invokeCommand).mockImplementation(async command => {
      if (command === "get_mod_capsule_pool") return packed as never;
      if (command === "unpack_mod_capsule") { expect(listenEvent).toHaveBeenCalled(); return await pending as never; }
      return undefined as never;
    });
    const onProcess = vi.fn(); render(<Harness onProcess={onProcess} />);
    await userEvent.click(await screen.findByRole("button", { name: "设置 mini" }));
    expect(screen.getByText("MPQ 压缩包")).toBeTruthy();
    expect(screen.queryByRole("button", { name: "加工功能" })).toBeNull();
    await userEvent.click(screen.getByRole("button", { name: "解压" }));
    expect(invokeCommand).toHaveBeenCalledWith("unpack_mod_capsule", { capsuleId: "scan:cn:mini" });
    expect(onProcess).not.toHaveBeenCalled();
    act(() => progressHandler?.({ payload: { capsule_id: "scan:cn:mini", task_id: 7, percent: 45, message: "正在解压 MPQ" } }));
    expect(screen.getByRole("progressbar").getAttribute("aria-valuenow")).toBe("45");
    await act(async () => { finish({ pool: unpacked, backup_path: "D:\\mods\\mini\\back\\id\\mini.mpq", escaped_name_count: 0 }); await pending; });
    const process = await screen.findByRole("button", { name: "加工功能" });
    expect(screen.queryByRole("button", { name: "解压" })).toBeNull();
    expect(onProcess).not.toHaveBeenCalled();
    expect(screen.getByText(/原 MPQ 备份/)).toBeTruthy();
    expect(unlisten).toHaveBeenCalledTimes(1);
    await userEvent.click(process);
    expect(onProcess).toHaveBeenCalledWith(unpacked.capsules[0]);
    expect(within(screen.getByRole("article", { name: "mini" })).getByText("-mod mini -txt")).toBeTruthy();
  });

  it("retains unpack on failure and rescans rather than marking the archive processed", async () => {
    vi.mocked(invokeCommand).mockImplementation(async command => {
      if (command === "unpack_mod_capsule") throw new Error("加工器不支持 MPQ 解压，请更新");
      return packed as never;
    });
    const onProcess = vi.fn(); render(<Harness onProcess={onProcess} />);
    await userEvent.click(await screen.findByRole("button", { name: "设置 mini" }));
    await userEvent.click(screen.getByRole("button", { name: "解压" }));
    await screen.findByText(/加工器不支持 MPQ 解压/);
    expect(screen.getByRole("button", { name: "解压" }).hasAttribute("disabled")).toBe(false);
    expect(screen.queryByRole("button", { name: "加工功能" })).toBeNull();
    expect(onProcess).not.toHaveBeenCalled();
    expect(vi.mocked(invokeCommand).mock.calls.filter(([c]) => c === "get_mod_capsule_pool")).toHaveLength(2);
  });

  it("cancels the matching task and keeps unpack available after recovery", async () => {
    let reject!: (reason: Error) => void;
    const pending = new Promise<ModUnpackResult>((_resolve, fail) => { reject = fail; });
    vi.mocked(invokeCommand).mockImplementation(async command => command === "unpack_mod_capsule" ? await pending as never : packed as never);
    render(<Harness onProcess={vi.fn()} />);
    await userEvent.click(await screen.findByRole("button", { name: "设置 mini" }));
    await userEvent.click(screen.getByRole("button", { name: "解压" }));
    act(() => progressHandler?.({ payload: { capsule_id: "scan:cn:mini", task_id: 52, percent: 20, message: "正在解压 MPQ" } }));
    await userEvent.click(screen.getByRole("button", { name: "取消解压" }));
    expect(invokeCommand).toHaveBeenCalledWith("cancel_task", { taskId: 52 });
    act(() => reject(new Error("已取消，原包已恢复")));
    await waitFor(() => expect(screen.getByRole("button", { name: "解压" }).hasAttribute("disabled")).toBe(false));
  });
});
