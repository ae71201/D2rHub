import { describe, expect, it } from "vitest";
import type { GlobalConfig } from "../store/types";
import { matchesRecognitionSource, recognitionSourceId } from "./recognitionSource";

describe("recognition source identity", () => {
  const config = { cn_game_path: "D:/Games/D2R/", global_game_path: "E:\\Games\\D2R", rune_audio_target_account: "main",
    rune_audio_external_target: { edition: "CN", mod_name: "audio" } } as GlobalConfig;
  it("keeps an installation stable across process restarts and rejects another installation", () => {
    expect(recognitionSourceId(config)).toBe("external:CN:d:\\games\\d2r");
    expect(matchesRecognitionSource(config, { source_id: recognitionSourceId(config), account_id: "" })).toBe(true);
    expect(matchesRecognitionSource(config, { source_id: "external:Global:e:\\games\\d2r", account_id: "main" })).toBe(false);
    expect(matchesRecognitionSource(config, { account_id: "main" })).toBe(false);
  });
  it("retains legacy account events when external mode is absent", () => {
    const accountConfig = { ...config, rune_audio_external_target: null };
    expect(recognitionSourceId(accountConfig)).toBe("main");
    expect(matchesRecognitionSource(accountConfig, { account_id: "main" })).toBe(true);
    expect(matchesRecognitionSource(accountConfig, { source_id: recognitionSourceId(config), account_id: "" })).toBe(false);
  });
});
