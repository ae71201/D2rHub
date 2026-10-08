import type { AccountMeta } from "../../store/types";
import { sortAccountsByCardOrder } from "../../utils/accountOrder";

/** Native window order takes precedence over historical launch records. */
export function layoutAccountLabels(accounts: readonly AccountMeta[], count: number, english = false, liveOrder?: readonly string[]): string[] {
  const liveIds = new Set(liveOrder?.map(id => id.toLowerCase()));
  const ordered = sortAccountsByCardOrder(accounts.filter(account => account.initialized || account.is_running || liveIds.has(account.id.toLowerCase())));
  const recorded = ordered.filter(account => account.is_running).sort((a, b) => {
    const timeA = Date.parse(a.last_launched_at ?? "");
    const timeB = Date.parse(b.last_launched_at ?? "");
    const valueA = Number.isFinite(timeA) ? timeA : Number.MAX_SAFE_INTEGER;
    const valueB = Number.isFinite(timeB) ? timeB : Number.MAX_SAFE_INTEGER;
    return valueA - valueB || a.order - b.order;
  });
  const byId = new Map(accounts.map(account => [account.id.toLowerCase(), account]));
  const running = liveOrder ? liveOrder.map(id => byId.get(id.toLowerCase()) ?? { id, display_name: id }) : recorded;
  const runningIds = new Set(running.map(account => account.id.toLowerCase()));
  const preview = [...running, ...ordered.filter(account => !runningIds.has(account.id.toLowerCase()))];
  return Array.from({ length: count }, (_, index) => preview[index]?.display_name.trim() || preview[index]?.id
    || (english ? `Unassigned window ${index + 1}` : `待分配窗口 ${index + 1}`));
}
