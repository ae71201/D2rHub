import type { BatchMode } from "../../hooks/useAccountBatch";
import { useMemo, type CSSProperties } from "react";
import "../../styles/accountStatusLight.css";

export interface AccountStatusLightProps {
  name: string;
  running: boolean;
  issue?: string | null;
  selected?: boolean;
  mode?: BatchMode;
  disabled?: boolean;
  uncertain?: boolean;
  paused?: boolean;
  onToggle?: () => void;
  onRepair?: () => void;
  activity?: string;
}
export function AccountStatusLight({ name, running, issue, selected, mode, disabled, uncertain, paused, onToggle, onRepair, activity }: AccountStatusLightProps) {
  // Align all selected lights to one clock without JS animation timers.
  const phase = useMemo(() => -(performance.now() % 1800), [selected]);
  const action = running ? "关闭" : "启动";
  const mismatch = !!mode && mode !== (running ? "close" : "launch");
  const unknown = uncertain || issue === undefined;
  const label = activity || (selected ? `待${action}` : issue ? `${running ? "运行中 · " : ""}配置异常` : unknown ? "状态待确认" : "");
  const title = unknown ? "状态待确认，请稍后重试" : selected ? `已选中待${action}，再次点击取消`
    : mismatch ? "请先取消当前选择" : issue && !running ? `配置异常：${issue}，点击配置`
    : `${running ? "运行中" : "未运行"}，点击选择${action}${issue ? `；配置异常：${issue}` : ""}`;
  return <div className="account-status-control" data-paused={paused || undefined}>
    {label && <span className="account-status-label">{label}</span>}
    <button type="button" className="account-status-button" title={title}
      aria-label={`${name}：${title}`} aria-pressed={!!selected}
      disabled={disabled || (unknown && !selected) || mismatch}
      onPointerDown={event => event.stopPropagation()}
      onMouseDown={event => event.stopPropagation()}
      onTouchStart={event => event.stopPropagation()}
      onKeyDown={event => { if (event.key === " " || event.key === "Enter") event.stopPropagation(); }}
      onClick={event => { event.stopPropagation(); if (issue && !running && !selected) onRepair?.(); else onToggle?.(); }}>
      <span className="account-status-dot" data-running={running} data-error={!!issue}
        style={{ "--status-phase": `${phase}ms` } as CSSProperties}
        data-selection={selected ? running ? "close" : "launch" : undefined} />
    </button>
  </div>;
}
