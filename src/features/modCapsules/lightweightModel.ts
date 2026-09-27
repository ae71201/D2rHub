export const LIGHTWEIGHT_PROFILES = [
  { id: "main", name: "LiteHub", label: "轻量", en: "Light", detail: "保留正常游戏所需的场景与角色显示，适合日常游玩", enDetail: "Keep the scene and character visuals needed for regular gameplay." },
  { id: "filler", name: "BoHub", label: "精简", en: "Reduced", detail: "仅保留人物显示，推荐用于 BO、强化等辅助角色", enDetail: "Only characters remain visible. Recommended for Battle Orders and Enchant support characters." },
  { id: "min", name: "NullHub", label: "极简", en: "Minimal", detail: "场景采用纯黑显示，仅适合站桩角色使用", enDetail: "Render the scene entirely black. Suitable only for stationary characters." },
] as const;
export type LightweightProfile = typeof LIGHTWEIGHT_PROFILES[number]["id"];
export function nextLightweightName(name: string, occupied: readonly string[]): string {
  const names = new Set(occupied.map((n) => n.toLowerCase()));
  let index = 2;
  while (names.has(`${name.slice(0, 58)}-${index}`.toLowerCase())) index += 1;
  return `${name.slice(0, 58)}-${index}`;
}
export function lightweightNameError(name: string, english: boolean): string | null {
  if (!/^[a-z0-9_-]{1,64}$/i.test(name) || /^(con|prn|aux|nul|com[1-9]|lpt[1-9])$/i.test(name)) {
    return english ? "Use 1–64 letters, numbers, hyphens or underscores; reserved names are not allowed." : "名称限 1–64 个英文字母、数字、短横线或下划线，不能使用系统保留名称。";
  }
  return null;
}
export interface LightweightContext { edition: string; game_directory: string; available: boolean; reason: string | null }
export interface LightweightResult { edition: string; profile: LightweightProfile; mod_name: string; mod_directory: string; launch_arguments: string; task_id: number }
