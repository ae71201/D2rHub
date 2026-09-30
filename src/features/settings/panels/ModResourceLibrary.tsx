import { Download, RefreshCw } from "lucide-react";
import { Button } from "../../../components/ui/Button";
import type { ModCapsuleController } from "../../modCapsules/useModCapsulePool";
import { ModResourceCard } from "../../modResources/ModResourceCard";
import { useModResources } from "../../modResources/useModResources";
import "./modResources.css";

const RESOURCE_ORDER = ["NullHub", "BoHub", "LiteHub"];

interface Props {
  edition: string;
  en: boolean;
  catalog?: ModCapsuleController;
  onBusy?: (busy: boolean) => void;
}

export function ModResourceLibrary({ edition, en, catalog, onBusy }: Props) {
  const controller = useModResources({ edition, onInstalled: catalog?.refresh, onBusy });
  const { data, checking, readError, feedback, taskError, runningElsewhere } = controller;
  const assets = (data?.catalog.assets ?? [])
    .filter(asset => asset.id !== "processor")
    .sort((left, right) => (RESOURCE_ORDER.indexOf(left.id) + 1 || 99) - (RESOURCE_ORDER.indexOf(right.id) + 1 || 99));

  return <section className="mod-resources" aria-label={en ? "Mod resources" : "Mod 资源下载"}>
    <header className="mod-resources-heading">
      <div>
        <h3><Download size={15} aria-hidden="true" />{en ? "Ready-to-use Mods" : "成品 Mod"}</h3>
        <p>{en ? "Choose a ready-to-use Mod. Custom processing is included with Hub." : "选择成品即可安装；自定义加工功能已随 Hub 内置。"}</p>
      </div>
      <Button size="sm" variant="ghost" disabled={controller.busy} loading={checking} onClick={() => void controller.refresh()}>
        <RefreshCw size={13} />{en ? "Check updates" : "检查更新"}
      </Button>
    </header>
    {checking && !data && <p role="status">{en ? "Checking installed resources…" : "正在检查本地资源…"}</p>}
    {data?.warning && <p className="resource-note" role="status">{data.warning}</p>}
    {taskError && <p className="resource-note" role="status">{en ? "Task progress is temporarily unavailable. Installation results will still appear here." : "暂时无法读取任务进度，安装结果仍会在这里显示。"}</p>}
    {data && <div className="resource-cards">
      {assets.map(asset => <ModResourceCard key={asset.id} asset={asset} state={data} controller={controller} en={en}
        installed={!!catalog?.pool?.capsules.some(mod => mod.edition === edition && mod.origin === "scanned" && mod.name.toLowerCase() === asset.id.toLowerCase())} />)}
      {!assets.length && <p role="status">{en ? "No resources are available in this catalog. Check updates to refresh it." : "当前目录暂无可用资源，可检查更新后重试。"}</p>}
    </div>}
    {runningElsewhere && <p role="status" className="resource-note">{en ? "Another game edition is installing resources. Follow it in Background tasks." : "另一个游戏版本正在安装资源，可在后台任务中查看进度。"}</p>}
    {readError && <p className="resource-error" role="alert">{readError}</p>}
    {feedback?.assetId === null && feedback.error && <p className="resource-error" role="alert">{feedback.error}</p>}
  </section>;
}
