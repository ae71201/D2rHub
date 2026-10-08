import type { GlobalConfig } from "../store/types";

export type ExternalAudioTarget = NonNullable<GlobalConfig["rune_audio_external_target"]>;
export type GameEdition = ExternalAudioTarget["edition"];

export function defaultExternalAudioTarget(config: GlobalConfig): ExternalAudioTarget {
  return { edition: config.cn_game_path?.trim() ? "CN" : "Global", mod_name: "" };
}

export function installationTargetId(edition: GameEdition): string {
  return `installation:${edition}`;
}

export function recognitionSourceId(config: GlobalConfig): string {
  const target = config.rune_audio_external_target;
  if (!target) return config.rune_audio_target_account;
  const path = target.edition === "CN" ? config.cn_game_path : config.global_game_path;
  return `external:${target.edition}:${path.trim().replaceAll("/", "\\").replace(/^\\\\\?\\/, "").replace(/\\+$/, "").toLowerCase()}`;
}

export function matchesRecognitionSource(config: GlobalConfig, event: { source_id?: string | null; account_id: string | null }): boolean {
  return config.rune_audio_external_target
    ? event.source_id === recognitionSourceId(config)
    : event.account_id === config.rune_audio_target_account && !!event.account_id;
}
