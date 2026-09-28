import { useCallback, useEffect, useRef, useState } from "react";
import { taskGateway } from "../../tasks/gateway";
import { subscribeBeforeReadingTasks } from "../../tasks/taskSync";
import type { TaskSnapshot } from "../../tasks/types";

interface Options { accountId: string; busy: boolean }

function matches(task: TaskSnapshot, accountId: string): boolean {
  return (task.kind === "audio-mod-prepare" || task.kind === "audio-mod-upgrade")
    && !!accountId.trim()
    && task.subject?.toLowerCase() === accountId.trim().toLowerCase();
}

/** A processing session can only cancel its account's actual processor task. */
export function useModPreparationTask({ accountId, busy }: Options) {
  const [currentTask, setCurrentTask] = useState<TaskSnapshot | null>(null);
  const [cancelError, setCancelError] = useState<string | null>(null);
  const [cancelling, setCancelling] = useState(false);
  const selected = useRef<TaskSnapshot | null>(null);
  const pendingCancel = useRef<number | null>(null);
  const revision = useRef(0);
  const context = useRef({ accountId, busy });
  context.current = { accountId, busy };

  useEffect(() => {
    const ticket = ++revision.current;
    let stop: (() => void) | undefined;
    let live = true;
    selected.current = null;
    pendingCancel.current = null;
    setCurrentTask(null);
    setCancelError(null);
    setCancelling(false);
    if (busy && accountId) {
      void subscribeBeforeReadingTasks(taskGateway, tasks => {
        if (!live || revision.current !== ticket) return;
        const matching = [...tasks.values()].filter(task => matches(task, accountId));
        let latest = matching.reduce<TaskSnapshot | null>((result, task) =>
          !result || task.task_id > result.task_id ? task : result, null);
        // The cancel response may arrive before its event. An unrelated event
        // must not restore the older pre-cancellation snapshot from this map.
        if (latest && selected.current?.task_id === latest.task_id
          && selected.current.revision > latest.revision) latest = selected.current;
        selected.current = latest;
        setCurrentTask(latest);
      }).then(unsubscribe => {
        if (live && revision.current === ticket) stop = unsubscribe;
        else unsubscribe();
      }).catch(error => {
        if (live && revision.current === ticket) setCancelError(String(error));
      });
    }
    return () => { live = false; revision.current += 1; stop?.(); };
  }, [accountId, busy]);

  const cancel = useCallback(async (): Promise<void> => {
    const task = selected.current;
    if (!context.current.busy || !task || !matches(task, context.current.accountId)
      || task.state !== "running" || task.cancel_requested || pendingCancel.current !== null) return;
    const ticket = revision.current;
    const applies = () => ticket === revision.current && context.current.busy
      && selected.current?.task_id === task.task_id && matches(task, context.current.accountId);
    pendingCancel.current = task.task_id;
    setCancelling(true);
    setCancelError(null);
    try {
      const updated = await taskGateway.cancel(task.task_id);
      if (applies() && updated.task_id === task.task_id
        && matches(updated, context.current.accountId)
        && updated.revision >= (selected.current?.revision ?? 0)) {
        selected.current = updated;
        setCurrentTask(updated);
      }
    } catch (error) {
      if (applies()) setCancelError(String(error));
    } finally {
      if (ticket === revision.current) {
        pendingCancel.current = null;
        setCancelling(false);
      }
    }
  }, []);

  return { currentTask, cancel, cancelError, cancelling };
}
