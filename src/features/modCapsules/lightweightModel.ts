export const LIGHTWEIGHT_PROFILES = [
  { id: "main", name: "LiteHub", label: "轻量", en: "Light", detail: "保留正常游戏所需的场景与角色显示，适合日常游玩", enDetail: "Keep the scene and character visuals needed for regular gameplay." },
  { id: "filler", name: "BoHub", label: "精简", en: "Reduced", detail: "仅保留人物显示，推荐用于 BO、强化等辅助角色", enDetail: "Only characters remain visible. Recommended for Battle Orders and Enchant support characters." },
  { id: "min", name: "NullHub", label: "极简", en: "Minimal", detail: "场景采用纯黑显示，仅适合站桩角色使用", enDetail: "Render the scene entirely black. Suitable only for stationary characters." },
] as const;
