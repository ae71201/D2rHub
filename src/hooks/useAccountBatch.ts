import { useCallback, useEffect, useRef, useState } from "react";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { invokeCommand } from "../platform/tauri";
import { useAccounts } from "../store/accounts";
import type { AccountMeta, GlobalConfig } from "../store/types";
import { requiresTokenMigration } from "../utils/regionPaths";

export type BatchMode = "launch" | "close" | null;
export interface BatchSelection { mode: BatchMode; ids: string[] }
export const emptySelection: BatchSelection = { mode: null, ids: [] };
export function toggleBatchSelection(selection: BatchSelection, id: string, running: boolean): BatchSelection {
  const mode = running ? "close" : "launch";
  if (selection.ids.includes(id)) {
    const ids = selection.ids.filter(value => value !== id);
    return ids.length ? { ...selection, ids } : emptySelection;
  }
  if (selection.mode && selection.mode !== mode) return selection;
  return { mode, ids: [...selection.ids, id] };
}
export function reconcileBatchSelection(selection: BatchSelection, accounts: AccountMeta[]): BatchSelection {
  const ids = selection.ids.filter(id => accounts.some(account => account.id === id
    && account.is_running === (selection.mode === "close")));
  return ids.length === selection.ids.length ? selection : ids.length ? { ...selection, ids } : emptySelection;
}

type Health = Record<string, string | null>;
let runningRequest: Promise<Set<string>> | null = null;
export function refreshBatchRunning(): Promise<Set<string>> {
  if (runningRequest) return runningRequest;
  runningRequest = invokeCommand<string[]>("refresh_account_running_state").then(values => {
    const ids = new Set(values);
    useAccounts.setState(state => {
      let changed = false;
      const accounts = state.accounts.map(account => {
        const running = ids.has(account.id);
        if (account.is_running === running) return account;
        changed = true;
        return { ...account, is_running: running, running_pid: running ? account.running_pid : null };
      });
      return changed ? { accounts } : state;
    });
    return ids;
  }).finally(() => { runningRequest = null; });
  return runningRequest;
}

export function useAccountBatch(active: boolean, suspended: boolean, config: GlobalConfig | null, busy: boolean) {
  const accounts = useAccounts(state => state.accounts);
  const [selection, setSelection] = useState<BatchSelection>(emptySelection);
  const [health, setHealth] = useState<Health>({});
  const [uncertain, setUncertain] = useState(true);
  const [motionPaused, setMotionPaused] = useState(true);
  const healthKey = JSON.stringify([config, accounts.map(({ is_running: _r, running_pid: _p, ...meta }) => meta)]);
  const keyRef = useRef(healthKey);
  keyRef.current = healthKey;
  const refreshRef = useRef<(() => Promise<void>) | null>(null);
  const clear = useCallback(() => setSelection(emptySelection), []);

  useEffect(() => { if (!active || suspended) clear(); }, [active, suspended, clear]);
  useEffect(() => {
    // Do not let a background refresh change the in-flight request's selection.
    if (!busy) setSelection(current => reconcileBatchSelection(current, accounts));
  }, [accounts, busy]);
  useEffect(() => {
    if (!selection.mode || busy) return;
    const escape = (event: KeyboardEvent) => {
      if (event.key !== "Escape" || event.defaultPrevented || document.querySelector('[role="dialog"]')) return;
      if (event.target instanceof HTMLElement && event.target.closest("input,textarea,select,[contenteditable=true]")) return;
      clear();
    };
    document.addEventListener("keydown", escape);
    return () => document.removeEventListener("keydown", escape);
  }, [selection.mode, busy, clear]);

  useEffect(() => {
    if (!active) { setMotionPaused(true); return; }
    setUncertain(true);
    let disposed = false;
    let pending: Promise<void> | null = null;
    let timer: ReturnType<typeof setTimeout> | undefined;
    let checkedKey = "";
    let healthAt = 0;
    const refresh = (): Promise<void> => {
      if (pending) return pending;
      clearTimeout(timer);
      pending = (async () => {
        try {
          const win = getCurrentWindow();
          const visible = !document.hidden && await win.isVisible() && !await win.isMinimized();
          if (disposed) return;
          setMotionPaused(!visible);
          if (!visible) return;
          await refreshBatchRunning();
          if (disposed) return;
          const key = keyRef.current;
          if (checkedKey !== key || Date.now() - healthAt > 30000) {
            const rows = await invokeCommand<{ account_id: string; error: string | null }[]>("inspect_account_launch_health");
            if (disposed) return;
            const next = Object.fromEntries(rows.map(row => [row.account_id, row.error]));
            setHealth(previous => Object.keys(previous).length === rows.length
              && rows.every(row => previous[row.account_id] === row.error) ? previous : next);
            checkedKey = key;
            healthAt = Date.now();
          }
          if (!disposed) setUncertain(false);
        } catch { if (!disposed) setUncertain(true); }
        finally {
          pending = null;
          if (!disposed) timer = setTimeout(() => { void refresh(); }, 3000);
        }
      })();
      return pending;
    };
    refreshRef.current = refresh;
    const wake = () => { if (document.hidden) setMotionPaused(true); else void refresh(); };
    document.addEventListener("visibilitychange", wake);
    window.addEventListener("focus", wake);
    void refresh();
    return () => {
      disposed = true;
      clearTimeout(timer);
      refreshRef.current = null;
      document.removeEventListener("visibilitychange", wake);
      window.removeEventListener("focus", wake);
    };
  }, [active]);
  useEffect(() => { void refreshRef.current?.(); }, [healthKey, busy]);

  const issue = (account: AccountMeta): string | null | undefined => {
    if (!account.initialized) return "账号尚未初始化，请补全配置";
    if (requiresTokenMigration(account.auth_mode, account.region, config)) return "请先迁移为 Token 直启";
    return health[account.id];
  };
  return {
    selection, setSelection, clear, issue, uncertain, motionPaused,
    refresh: () => refreshRef.current?.() ?? Promise.resolve(),
    toggle: (account: AccountMeta) => {
      if (busy || suspended) return;
      if (!selection.ids.includes(account.id) && (uncertain || (!account.is_running && issue(account) !== null))) return;
      setSelection(current => toggleBatchSelection(current, account.id, account.is_running));
    },
  };
}
