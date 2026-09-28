import { Button } from "../../../components/ui/Button";
import { ModCatalogManager } from "../../settings/panels/ModCatalogManager";
import { ModDownloadsPage } from "../../settings/panels/ModResourceLibrary";
import { ModProcessingPanel } from "../../settings/panels/ModProcessingPanel";
import type { ModWorkflowController } from "./useModWorkflow";

export function ModWorkspace({ workflow }: { workflow: ModWorkflowController }) {
  const { view, draft, actions, en } = workflow;
  if (view === "resources" && draft) return <ModDownloadsPage edition={draft.edition} en={en} catalog={workflow.catalog} processorOnly
    onBack={actions.back} onEditionChange={() => { /* Resources belong to the processing target edition. */ }} />;
  if (view === "processing" && draft) return <ModProcessingPanel workflow={workflow} />;
  return <>
    {draft && <div className="flex items-center justify-end pb-2">
      <Button size="sm" variant="ghost" onClick={actions.resume}>{en ? "Resume processing draft" : "继续未完成的加工"}</Button>
    </div>}
    <ModCatalogManager catalog={workflow.catalog} accounts={workflow.accounts} language={en ? "en-US" : "zh-CN"}
      minimalMode={workflow.minimalMode} autoOpenAdd={workflow.openAdd} edition={workflow.libraryEdition}
      onEditionChange={actions.setLibraryEdition}
      onProcess={capsule => actions.requestProcessing({ origin: "library", edition: capsule.edition === "Global" ? "Global" : "CN",
        source: { name: capsule.name, processed: capsule.processed || capsule.update_required } })}
      onCreate={edition => actions.requestProcessing({ origin: "library", edition: edition === "Global" ? "Global" : "CN" })} />
  </>;
}
