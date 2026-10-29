import { describe, it, expect } from "vitest";
import { switchPrimary } from "./roles";
describe("leader switching", () => {
  const original = { primary_account_id: "1", follower_account_ids: ["2", "3", "4"] };
  it("swaps an existing follower without changing other followers", () => {
    expect(switchPrimary(original, "2")).toEqual({ primary_account_id: "2", follower_account_ids: ["1", "3", "4"] });
    expect(original.follower_account_ids).toEqual(["2", "3", "4"]);
  });
  it("keeps all former participants when choosing an outside leader", () => {
    expect(switchPrimary(original, "5").follower_account_ids).toEqual(["1", "2", "3", "4"]);
  });
  it("keeps order on a no-op and on a swap back", () => {
    expect(switchPrimary(original, "1")).toEqual(original);
    expect(switchPrimary(switchPrimary(original, "2"), "1")).toEqual(original);
  });
});
