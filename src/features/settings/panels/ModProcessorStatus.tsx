import { useEffect, useRef, useState } from "react";
import { Button } from "../../../components/ui/Button";
import { modResourcesGateway } from "../../modResources/gateway";
import type { ModProcessorState } from "../../modResources/types";
import "./modResources.css";

interface Props { edition: string; en: boolean; onReady: (ready: boolean | null) => void; onManage: () => void }
export function ModProcessorStatus({ edition, en, onReady, onManage }: Props) {
  const [status, setStatus] = useState<ModProcessorState | null>(null);
  const [error, setError] = useState<string | null>(null);
  const readyCallback = useRef(onReady);
  readyCallback.current = onReady;
  useEffect(() => {
    let live = true;
    setStatus(null); setError(null); readyCallback.current(null);
    // Only inspect local availability. The app's daily update check owns automatic networking.
    void modResourcesGateway.read(edition)
      .then(value => { if (live) { setStatus(value.processor); readyCallback.current(value.processor.ready && !value.processor.update_available && !value.processor.blocking_reason); } })
      .catch(cause => { if (live) { setError(String(cause)); readyCallback.current(false); } });
    return () => { live = false; };
  }, [edition]);
  if (status?.ready && !status.update_available && !status.blocking_reason) return null;
  return <div className="processor-status" role="status">
    <span>{error ?? (status ? status.blocking_reason ?? (en
      ? `Processing blocked. Installed processor: ${status.installed_version ?? "not installed"}; required: ${status.recommended_version}. Open Downloads & updates to install the paired version. Hub and processor must recognize each other before processing.`
      : `已禁止加工。当前加工器：${status.installed_version ?? "未安装"}；所需配套版本：${status.recommended_version}。请前往“下载与更新”安装配套版本，Hub 与加工器互认成功后才能加工。`)
      : (en ? "Checking local processor…" : "正在读取本地加工器状态…"))}</span>
    {(status || error) && <Button size="sm" variant="secondary" onClick={onManage}>{en ? "Downloads & updates" : "下载与更新"}</Button>}
  </div>;
}
