import { beforeEach, expect, it, vi } from "vitest";
import type { AccountMeta } from "./types";
const invoke = vi.hoisted(() => vi.fn());
vi.mock("../platform/tauri", () => ({ invokeCommand: invoke }));
import { useAccounts } from "./accounts";
const account = (region: string) => ({ id: "one", region }) as AccountMeta;
function pending<T>() { let resolve!: (v: T) => void; let reject!: (e: Error) => void; const promise = new Promise<T>((a, b) => { resolve = a; reject = b; }); return { promise, resolve, reject }; }
beforeEach(() => { invoke.mockReset(); useAccounts.setState({ accounts: [account("KR")], loading: false, error: null }); });
it("ignores an older list result after a newer refresh", async () => {
  const old = pending<AccountMeta[]>(), latest = pending<AccountMeta[]>();
  invoke.mockReturnValueOnce(old.promise).mockReturnValueOnce(latest.promise);
  const a = useAccounts.getState().loadAccounts(), b = useAccounts.getState().loadAccounts();
  latest.resolve([account("EU")]); await b;
  old.resolve([account("KR")]); await a;
  expect(useAccounts.getState().accounts[0].region).toBe("EU");
});
it("does not roll back a local metadata update while loading", async () => {
  const read = pending<AccountMeta[]>(); invoke.mockReturnValueOnce(read.promise);
  const loading = useAccounts.getState().loadAccounts();
  useAccounts.getState().updateAccount(account("NA"));
  read.resolve([account("KR")]); await loading;
  expect(useAccounts.getState().accounts[0].region).toBe("NA");
  expect(useAccounts.getState().loading).toBe(false);
});
it("an obsolete failure cannot replace newer success", async () => {
  const old = pending<AccountMeta[]>(), latest = pending<AccountMeta[]>();
  invoke.mockReturnValueOnce(old.promise).mockReturnValueOnce(latest.promise);
  const a = useAccounts.getState().loadAccounts(), b = useAccounts.getState().loadAccounts();
  latest.resolve([account("EU")]); await b;
  old.reject(new Error("outdated")); await a;
  expect(useAccounts.getState().error).toBeNull();
});
