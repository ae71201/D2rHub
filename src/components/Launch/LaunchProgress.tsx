import { ChevronDown, Trash2 } from "lucide-react";
import { useState, useEffect, useRef } from "react";
import type { LaunchLog } from "../../store/launch";
import type { AccountMeta } from "../../store/types";

interface Props {
  accounts: AccountMeta[];
  logs: LaunchLog[];
  onClear: () => void;
  revealRevision?: number;
  language?: string;
}

export function LaunchProgressView({ accounts, logs, onClear, revealRevision = 0, language }: Props) {
  const english = language === "en-US";
  const [expanded, setExpanded] = useState(false);
  const userCollapsed = useRef(false);

  useEffect(() => {
    if (revealRevision > 0) { userCollapsed.current = false; setExpanded(true); }
  }, [revealRevision]);

  // New output reveals the timeline; it stays readable until the user closes it.
  useEffect(() => {
    if (logs.length === 0) userCollapsed.current = false;
    if (logs.length > 0 && !userCollapsed.current) {
      setExpanded(true);
    }
  }, [logs.length]);

  if (logs.length === 0) return null;

  const getAccountName = (id: string) => {
    const a = accounts.find(a => a.id === id);
    return a?.display_name || id;
  };

  const statusColor = (status: string) => {
    switch (status) {
      case "ok": return "var(--success)";
      case "error": return "var(--error)";
      case "warning": return "var(--warning)";
      default: return "var(--text-muted)";
    }
  };

  return (
    <div className="mx-5 mb-4 account-line overflow-hidden transition-all duration-300">
      <div className="flex items-center pr-3">
      <button
        type="button"
        aria-expanded={expanded}
        onClick={() => setExpanded(prev => { userCollapsed.current = prev; return !prev; })}
        className="w-full flex items-center gap-2 px-3.5 py-2 text-left hover:bg-surface-hover transition-colors"
      >
        <ChevronDown size={12}
          className={"text-text-muted transition-transform duration-200 " + (expanded ? "rotate-0" : "-rotate-90")} />
        <span className="text-xs font-semibold text-text-secondary flex-1">
          {english ? "Activity log" : "运行日志"}
          <span className="text-text-muted font-normal ml-1.5">({logs.length})</span>
          {logs.some(log => log.status === "error") && <span className="text-error font-normal ml-2">{english ? "Errors recorded" : "有失败记录"}</span>}
        </span>
      </button>
        <button type="button" onClick={onClear}
          className="icon-btn w-6 h-6" title={english ? "Clear log" : "清除日志"}>
          <Trash2 size={10} />
        </button>
      </div>

      <div
        className="transition-all duration-300 ease-in-out grid"
        style={{
          gridTemplateRows: expanded ? "1fr" : "0fr",
          opacity: expanded ? 1 : 0,
        }}
      >
        <div className="overflow-hidden">
          <div className="max-h-44 overflow-auto px-3.5 pb-2.5 space-y-px">
            {[...logs].reverse().map((log, i) => (
              <div key={i} className="flex items-start gap-2 py-1 text-[10px] font-mono leading-relaxed">
                <span className="text-text-muted shrink-0 w-12">
                  {new Date(log.timestamp).toLocaleTimeString("zh-CN", { hour12: false })}
                </span>
                <span className="shrink-0 w-1 h-1 rounded-full mt-[5px]"
                  style={{ background: statusColor(log.status) }} />
                <span className="text-text-muted shrink-0 min-w-[48px]">
                  {getAccountName(log.account_id)}
                </span>
                <span className="text-text-secondary flex-1 break-all">{log.message}</span>
              </div>
            ))}
          </div>
        </div>
      </div>
    </div>
  );
}
