import { describe, it, expect } from "vitest";
import { copyRoomRule, parseRoomRule } from "./ruleShare";
import type { RoomAutomationConfig } from "./types";
describe("shared room rules", () => {
  it("copies only portable fields and round trips the next room", () => {
    const rule = { name_prefix: "巴尔-", password: "abc", next_sequence: 18, sequence_width: 3 };
    const text = copyRoomRule({ ...rule, primary_account_id: "private-account", shortcut: "Ctrl+R" } as RoomAutomationConfig);
    expect(parseRoomRule(text)).toEqual(rule);
    expect(text).not.toContain("private-account");
    expect(text).not.toContain("Ctrl+R");
  });
  it("rejects malformed, future, overflowing and oversized rules", () => {
    const base = { format: "d2rhub-room-rule", version: 1, name_prefix: "run-", password: "", next_sequence: 1, sequence_width: 3 };
    for (const patch of [{ version: 2 }, { next_sequence: -1 }, { next_sequence: 4294967295 }, { sequence_width: 7 }, { name_prefix: "x".repeat(16) }, { password: [] }]) {
      expect(() => parseRoomRule(JSON.stringify({ ...base, ...patch }))).toThrow();
    }
    expect(() => parseRoomRule("null")).toThrow();
  });
});
