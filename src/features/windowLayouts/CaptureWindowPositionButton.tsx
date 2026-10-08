import { useRef, useState } from "react";
import { MapPin } from "lucide-react";
import { useAccounts } from "../../store/accounts";
import type { AccountMeta } from "../../store/types";
import { showToast } from "../../components/ui/Toast";

export function CaptureWindowPositionButton({ accountId, activate = true, english, beforeCapture, onCaptured }: {
  accountId: string;
  activate?: boolean;
  english?: boolean;
  beforeCapture?: () => Promise<boolean>;
  onCaptured?: (account: AccountMeta) => void;
}) {
  const [busy, setBusy] = useState(false);
  const busyRef = useRef(false);
  const capture = async () => {
    if (busyRef.current) return;
    busyRef.current = true;
    setBusy(true);
    try {
      if (beforeCapture && !(await beforeCapture())) return;
      const account = await useAccounts.getState().captureWindowPosition(accountId, activate);
      if (account) {
        onCaptured?.(account);
        showToast("success", english ? "Current position saved as a new account preset" : "已将当前位置存为此账号的新位置配置");
      }
    } catch (error) { showToast("error", `${english ? "Unable to save position" : "存储当前位置失败"}：${error}`); }
    finally { busyRef.current = false; setBusy(false); }
  };
  return <button type="button" className="control-btn capture-position-button" disabled={busy} data-i18n-skip
    title={english ? "Find this account's game window by PID or nickname and save its coordinates" : "按 PID 或账号昵称查找游戏窗口，将当前坐标保存为一个新位置配置"}
    onClick={event => { event.stopPropagation(); void capture(); }}>
    <MapPin size={13} aria-hidden="true" />{busy ? (english ? "Saving…" : "正在获取…") : (english ? "Save current position" : "存储当前位置")}
  </button>;
}
