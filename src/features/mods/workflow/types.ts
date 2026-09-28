import type { AudioModSetupState } from "../../../store/types";
import type { AudioModFeatureSelection } from "../featureContract";

export type ModEdition = "CN" | "Global";
export type ModWorkflowOrigin = "library" | "recognition" | "room-automation";
export type ModRecipe =
  | { kind: "create"; source: string | null; name: string }
  | { kind: "augment"; modName: string };

/** An explicit user intent. Background inspection never creates or changes it. */
export interface ModProcessingRequest {
  origin: ModWorkflowOrigin;
  accountId?: string;
  edition?: ModEdition;
  source?: { name: string; processed: boolean };
  autoStart?: boolean;
}

export interface ModProcessingDraft {
  origin: ModWorkflowOrigin;
  accountId: string;
  edition: ModEdition;
  recipe: ModRecipe;
  features: AudioModFeatureSelection;
}

export interface ModAppliedResult {
  origin: ModWorkflowOrigin;
  accountId: string;
  state: AudioModSetupState;
}
