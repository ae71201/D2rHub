import { act, cleanup, renderHook, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { taskGateway } from "../../tasks/gateway";
import type { TaskSnapshot } from "../../tasks/types";
import { useModPreparationTask } from "./useModPreparationTask";

vi.mock("../../tasks/gateway", () => ({ taskGateway: {
  subscribe: vi.fn(), list: vi.fn(), cancel: vi.fn(),
} }));

const task = (overrides: Partial<TaskSnapshot> = {}): TaskSnapshot => ({
  task_id: 1, revision: 1, kind: "audio-mod-prepare", subject: "main",
  conflict_key: "audio-mod-build", state: "running", progress: 12,
  step: "generating", message: "Working", error_code: null,
  cancel_requested: false, retryable: true, retry_of: null, started_at_ms: 1, finished_at_ms: null,
  ...overrides,
});
let emit: (snapshot: TaskSnapshot) => void;
let stop: ReturnType<typeof vi.fn>;
beforeEach(() => {
  vi.mocked(taskGateway.subscribe).mockReset();
  vi.mocked(taskGateway.list).mockReset().mockResolvedValue([]);
  vi.mocked(taskGateway.cancel).mockReset();
  stop = vi.fn();
  vi.mocked(taskGateway.subscribe).mockImplementation(async listener => { emit = listener; return stop; });
});
afterEach(cleanup);

describe("Processing task cancellation", () => {
  it("subscribes before reading and selects only the current account processor task", async () => {
    vi.mocked(taskGateway.list).mockResolvedValue([
      task({ task_id: 8, kind: "mod-resource-install" }),
      task({ task_id: 7, subject: "other" }),
      task({ task_id: 6, kind: "audio-mod-upgrade", subject: "MAIN" }),
      task({ task_id: 5 }),
    ]);
    const { result } = renderHook(() => useModPreparationTask({ accountId: "main", busy: true }));
    await waitFor(() => expect(result.current.currentTask?.task_id).toBe(6));
    expect(vi.mocked(taskGateway.subscribe).mock.invocationCallOrder[0]).toBeLessThan(vi.mocked(taskGateway.list).mock.invocationCallOrder[0]);
    vi.mocked(taskGateway.cancel).mockResolvedValue(task({ task_id: 6, revision: 2, kind: "audio-mod-upgrade", cancel_requested: true }));
    await act(() => result.current.cancel());
    expect(taskGateway.cancel).toHaveBeenCalledWith(6);
    expect(result.current.currentTask?.cancel_requested).toBe(true);
    act(() => emit(task({ task_id: 9, kind: "mod-resource-install" })));
    expect(result.current.currentTask?.cancel_requested).toBe(true);
  });

  it("never cancels unrelated or finished tasks and does not subscribe while idle", async () => {
    const { result, rerender } = renderHook(({ busy }) => useModPreparationTask({ accountId: "main", busy }), { initialProps: { busy: false } });
    await act(() => result.current.cancel());
    expect(taskGateway.subscribe).not.toHaveBeenCalled();
    vi.mocked(taskGateway.list).mockResolvedValue([task({ state: "succeeded" }), task({ task_id: 2, subject: "other" })]);
    rerender({ busy: true });
    await waitFor(() => expect(result.current.currentTask?.state).toBe("succeeded"));
    await act(() => result.current.cancel());
    expect(taskGateway.cancel).not.toHaveBeenCalled();
  });

  it("deduplicates pending cancellation and never replaces a newer terminal event", async () => {
    vi.mocked(taskGateway.list).mockResolvedValue([task()]);
    let finish!: (value: TaskSnapshot) => void;
    vi.mocked(taskGateway.cancel).mockReturnValue(new Promise(resolve => { finish = resolve; }));
    const { result } = renderHook(() => useModPreparationTask({ accountId: "main", busy: true }));
    await waitFor(() => expect(result.current.currentTask).not.toBeNull());
    let cancellation!: Promise<void>;
    act(() => { cancellation = result.current.cancel(); void result.current.cancel(); });
    expect(taskGateway.cancel).toHaveBeenCalledTimes(1);
    expect(result.current.cancelling).toBe(true);
    act(() => emit(task({ revision: 4, state: "cancelled" })));
    await act(async () => { finish(task({ revision: 2, cancel_requested: true })); await cancellation; });
    expect(result.current.currentTask?.state).toBe("cancelled");
    expect(result.current.cancelling).toBe(false);
  });

  it("surfaces cancellation rejection without an unhandled promise and allows retry", async () => {
    vi.mocked(taskGateway.list).mockResolvedValue([task()]);
    vi.mocked(taskGateway.cancel).mockRejectedValueOnce(new Error("cancel unavailable"));
    const { result } = renderHook(() => useModPreparationTask({ accountId: "main", busy: true }));
    await waitFor(() => expect(result.current.currentTask).not.toBeNull());
    await act(() => result.current.cancel());
    expect(result.current.cancelError).toContain("cancel unavailable");
    expect(result.current.cancelling).toBe(false);
    vi.mocked(taskGateway.cancel).mockResolvedValue(task({ revision: 2, cancel_requested: true }));
    await act(() => result.current.cancel());
    expect(result.current.cancelError).toBeNull();
  });

  it("cleans listeners and ignores late cancellation errors after the target changes", async () => {
    vi.mocked(taskGateway.list).mockResolvedValue([task()]);
    let fail!: (error: Error) => void;
    vi.mocked(taskGateway.cancel).mockReturnValue(new Promise((_, reject) => { fail = reject; }));
    const { result, rerender, unmount } = renderHook(({ accountId }) => useModPreparationTask({ accountId, busy: true }), { initialProps: { accountId: "main" } });
    await waitFor(() => expect(result.current.currentTask).not.toBeNull());
    let cancellation!: Promise<void>;
    act(() => { cancellation = result.current.cancel(); });
    rerender({ accountId: "other" });
    await act(async () => { fail(new Error("old failure")); await cancellation; });
    expect(result.current.cancelError).toBeNull();
    expect(result.current.currentTask).toBeNull();
    expect(stop).toHaveBeenCalled();
    unmount();
    expect(stop).toHaveBeenCalledTimes(2);
  });
});
