import { useState } from "react";
import { RefreshCw } from "lucide-react";
import { Button } from "../../../components/ui/Button";
import type { ModCapsuleController } from "../../modCapsules/useModCapsulePool";
import { ModResourceCard } from "../../modResources/ModResourceCard";
import { useModResources } from "../../modResources/useModResources";
import "./modResources.css";

const RESOURCE_ORDER = ["NullHub", "BoHub", "LiteHub", "processor"];

interface Props {
  edition: string;
  en: boolean;
  catalog?: ModCapsuleController;
  processorOnly?: boolean;
  onBusy?: (busy: boolean) => void;
}

export function ModResourceLibrary({ edition, en, catalog, processorOnly = false, onBusy }: Props) {
  const controller = useModResources({ edition, onInstalled: catalog?.refresh, onBusy });
  const { data, checking, readError, feedback, taskError, runningElsewhere } = controller;
  const assets = (data?.catalog.assets ?? [])
    .filter(asset => !processorOnly || asset.id === "processor")
    .sort((left, right) => (RESOURCE_ORDER.indexOf(left.id) + 1 || 99) - (RESOURCE_ORDER.indexOf(right.id) + 1 || 99));

  return <section className="mod-resources" aria-label={en ? "Mod resources" : "Mod 资源下载"}>
    <header>
      <div>
        {processorOnly && <h3>{en ? "Mod processor" : "Mod 加工器"}</h3>}
        <p>{processorOnly
          ? (en ? "Install the compatible processor to add game features to your Mods." : "安装兼容的加工器，为 Mod 添加所需的游戏功能。")
          : (en ? "Choose a ready-to-use Mod. The processor is only needed for custom features." : "选择成品即可安装；需要自定义游戏功能时，再添加加工器。")}</p>
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

export function ModDownloadsPage({ edition, en, catalog, processorOnly = false, onBack, onEditionChange }: Props & { onBack: () => void; onEditionChange: (edition: "CN" | "Global") => void }) {
  const [busy, setBusy] = useState(false);
  return <div className="mod-downloads-page">
    <header className="mod-processing-header"><div><h2>{processorOnly ? (en ? "Install Mod processor" : "安装 Mod 加工器") : (en ? "Mod downloads & updates" : "Mod 下载与更新")}</h2>
      <p>{processorOnly ? (en ? "Your processing draft is saved. Return after the processor is ready." : "加工草稿已保留，安装就绪后返回继续。") : (en ? "Download Mods and manage the independent processor." : "下载 Mod，管理独立加工器及已安装资源。")}</p></div>
      <Button size="sm" variant="ghost" onClick={onBack}>{processorOnly ? (en ? "Back to processing" : "返回加工") : (en ? "Back" : "返回")}</Button></header>
    {!processorOnly && <div className="mod-catalog-editions" role="tablist" aria-label={en ? "Game edition" : "游戏版本"}>
      {(["CN", "Global"] as const).map(value => <button key={value} role="tab" type="button" aria-selected={edition === value} disabled={busy} onClick={() => onEditionChange(value)}>{value === "CN" ? (en ? "China" : "国服") : (en ? "Global" : "国际服")}</button>)}
    </div>}
    <ModResourceLibrary edition={edition} en={en} catalog={catalog} onBusy={setBusy} processorOnly={processorOnly} />
  </div>;
}
