import { afterEach, describe, expect, it, vi } from "vitest";
import { createChatterPicker, type ChatterHistory } from "./chatter";
import { CHATTER_LINES } from "./chatterData";

afterEach(() => vi.restoreAllMocks());

describe("pet wardrobe chatter", () => {
  it.each(["gentle", "snarky"] as const)("uses outfit topics with %s tone without recent repeats", tone => {
    let now = 0;
    vi.spyOn(performance, "now").mockImplementation(() => now);
    // Pick the themed pool and its first eligible entry each time.
    vi.spyOn(Math, "random").mockReturnValue(0);
    const history: ChatterHistory = { recent: [], nextAt: 0 };
    const picker = createChatterPicker({ wings: "feather-wings", aura: "moon-aura" }, tone, history);
    const seen = new Set<string>();
    for (let i = 0; i < 9; i++) {
      const line = picker(i % 2 === 1);
      const entry = CHATTER_LINES.find(entry => entry.zh === line || entry.en === line)!;
      expect(entry).toBeTruthy();
      expect(["wings", "aura", "paladin", "stargazer"]).toContain(entry.topic);
      expect(entry.tone).toBe(tone);
      expect(seen.has(entry.id)).toBe(false);
      seen.add(entry.id);
      expect(picker(false)).toBeNull();
      now += 45_000;
    }
    expect(history.recent).toHaveLength(8);
  });

  it.each(["wings", "aura"] as const)("selects the new %s topic in English", slot => {
    vi.spyOn(performance, "now").mockReturnValue(0);
    vi.spyOn(Math, "random").mockReturnValueOnce(0).mockReturnValueOnce(0).mockReturnValueOnce(0.6);
    const picker = createChatterPicker(slot === "wings" ? { wings: "feather-wings" } : { aura: "moon-aura" }, "gentle");
    const line = picker(true);
    expect(CHATTER_LINES.some(entry => entry.en === line && entry.topic === slot && entry.tone === "gentle")).toBe(true);
  });

  it("keeps cooldown and recent lines when switching outfits and tones", () => {
    let now = 0;
    vi.spyOn(performance, "now").mockImplementation(() => now);
    vi.spyOn(Math, "random").mockReturnValue(0);
    const history: ChatterHistory = { recent: [], nextAt: 0 };
    const wings = createChatterPicker({ wings: "bat-wings" }, "gentle", history);
    expect(wings(false)).toBeTruthy();
    const recent = history.recent[0];
    const aura = createChatterPicker({ aura: "dawn-aura" }, "snarky", history);
    expect(aura(true)).toBeNull();
    expect(history.recent).toEqual([recent]);
    now = 45_000;
    const line = aura(true);
    expect(CHATTER_LINES.some(entry => entry.en === line && entry.tone === "snarky" && ["aura", "paladin"].includes(entry.topic))).toBe(true);
  });
});
