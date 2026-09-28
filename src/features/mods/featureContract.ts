import type { AudioModFeatureGroup } from "../../store/types";

export interface AudioModPrepareProgress {
  account_id: string;
  phase: string;
  percent: number;
  message: string;
}

export interface AudioModPrepareResult {
  account_id: string;
  mod_name: string;
  mod_directory: string;
  launch_arguments: string;
  source_mod_name: string | null;
  feature_groups: AudioModFeatureGroup[];
}

export interface AudioModFeatureSelection {
  includeAudioTelemetry: boolean;
  includeRoomTools: boolean;
  includeEscNextGame?: boolean;
  includeAutoExitOnDeath: boolean;
}

export type AudioModProcessingPurpose = "recognition" | "room-tools" | "manage";

export const AUDIO_TELEMETRY_FEATURE_ID = "audio_telemetry";
export const IN_GAME_ROOM_TOOLS_FEATURE_ID = "in_game_room_tools";
export const ESC_NEXT_GAME_FEATURE_ID = "esc_next_game";
export const AUTO_EXIT_ON_DEATH_FEATURE_ID = "auto_exit_on_death";

export function audioModFeatureDefaultsForPurpose(
  purpose: AudioModProcessingPurpose,
): AudioModFeatureSelection {
  return {
    includeAudioTelemetry: purpose === "recognition",
    includeRoomTools: purpose === "room-tools",
    includeEscNextGame: false,
    includeAutoExitOnDeath: false,
  };
}

export function audioModFeatureInvokeOptions(
  selection: AudioModFeatureSelection,
): AudioModFeatureSelection {
  return {
    includeAudioTelemetry: selection.includeAudioTelemetry,
    includeRoomTools: selection.includeRoomTools,
    includeEscNextGame: selection.includeEscNextGame ?? false,
    includeAutoExitOnDeath: selection.includeAutoExitOnDeath,
  };
}

export function hasSelectedAudioModFeature(
  selection: AudioModFeatureSelection,
): boolean {
  return selection.includeAudioTelemetry
    || selection.includeRoomTools
    || selection.includeEscNextGame
    || selection.includeAutoExitOnDeath;
}

export function selectedAudioModFeatureAddsCapability(
  selection: AudioModFeatureSelection,
  installedGroups: readonly string[],
): boolean {
  return (
    (selection.includeAudioTelemetry && !installedGroups.includes(AUDIO_TELEMETRY_FEATURE_ID))
    || (selection.includeRoomTools && !installedGroups.includes(IN_GAME_ROOM_TOOLS_FEATURE_ID))
    || (selection.includeEscNextGame && !installedGroups.includes(ESC_NEXT_GAME_FEATURE_ID))
    || (selection.includeAutoExitOnDeath
      && !installedGroups.includes(AUTO_EXIT_ON_DEATH_FEATURE_ID))
  );
}

export function hasAudioTelemetry(
  groups: readonly string[] | readonly AudioModFeatureGroup[],
): boolean {
  return groups.some((group) => (
    typeof group === "string" ? group : group.id
  ) === AUDIO_TELEMETRY_FEATURE_ID);
}
