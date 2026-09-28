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
      .then(value => { if (live) { setStatus(value.processor); readyCallback.current(value.processor.ready); } })
      .catch(cause => { if (live) { setError(String(cause)); readyCallback.current(false); } });
    return () => { live = false; };
  }, [edition]);
  if (status?.ready && !status.update_available) return null;
  return <div className="processor-status" role="status">
    <span>{error ?? (status ? status.ready
      ? (en ? "A processor update is available. You can continue processing." : "加工器有新版本，当前版本仍可继续加工。")
      : (en ? "Install a compatible processor before processing." : "加工前需要安装兼容的加工器。")
      : (en ? "Checking local processor…" : "正在读取本地加工器状态…"))}</span>
    {(status || error) && <Button size="sm" variant="secondary" onClick={onManage}>{en ? "Downloads & updates" : "下载与更新"}</Button>}
  </div>;
}
