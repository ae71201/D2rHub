import { useState } from "react";
import { Button } from "../../../components/ui/Button";
import { showToast } from "../../../components/ui/Toast";
import { modResourcesGateway } from "../../modResources/gateway";
import { ModCatalogManager } from "../../settings/panels/ModCatalogManager";
import { ModDownloadsPage } from "../../settings/panels/ModResourceLibrary";
import { ModProcessingPanel } from "../../settings/panels/ModProcessingPanel";
import type { ModWorkflowController } from "./useModWorkflow";

export function ModWorkspace({ workflow }: { workflow: ModWorkflowController }) {
  const { view, draft, actions, en } = workflow;
  const [processorEdition, setProcessorEdition] = useState<"CN" | "Global" | null>(null);
  if (processorEdition) return <ModDownloadsPage edition={processorEdition} en={en} catalog={workflow.catalog}
    onBack={() => setProcessorEdition(null)} onEditionChange={setProcessorEdition} />;
  if (view === "resources" && draft) return <ModDownloadsPage edition={draft.edition} en={en} catalog={workflow.catalog} processorOnly
    onBack={actions.back} onEditionChange={() => { /* Resources belong to the processing target edition. */ }} />;
  if (view === "processing" && draft) return <ModProcessingPanel workflow={workflow} />;
  return <>
    {draft && <div className="mod-draft-notice">
      <Button size="sm" variant="ghost" onClick={actions.resume}>{en ? "Continue editing draft" : "继续编辑加工草稿"}</Button>
      <Button size="sm" variant="ghost" disabled={workflow.busy} onClick={actions.discardDraft}>{en ? "Discard draft" : "放弃草稿"}</Button>
    </div>}
    <ModCatalogManager catalog={workflow.catalog} accounts={workflow.accounts} language={en ? "en-US" : "zh-CN"}
      minimalMode={workflow.minimalMode} autoOpenAdd={workflow.openAdd} edition={workflow.libraryEdition}
      onEditionChange={actions.setLibraryEdition}
      onProcess={async capsule => {
        const edition = capsule.edition === "Global" ? "Global" : "CN";
        try {
          const local = await modResourcesGateway.read(edition);
          if (!local.processor.ready) { setProcessorEdition(edition); return; }
          actions.requestProcessing({ origin: "library", edition,
            source: { name: capsule.name, processed: capsule.processed || capsule.update_required } });
        } catch (error) { showToast("error", String(error)); }
      }}
      onCreate={edition => actions.requestProcessing({ origin: "library", edition: edition === "Global" ? "Global" : "CN" })} />
  </>;
}
