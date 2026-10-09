import { sortAccountsByCardOrder } from "./accountOrder";

function assert(condition: boolean, message: string) {
  if (!condition) throw new Error(`FAIL: ${message}`);
  console.log(`PASS: ${message}`);
}

const creationOrder = [
  { id: "created-first", order: 2 },
  { id: "created-second", order: 0 },
  { id: "created-third", order: 1 },
];
const ordered = sortAccountsByCardOrder(creationOrder);

assert(
  ordered.map((account) => account.id).join(",") === "created-second,created-third,created-first",
  "shortcut account names follow the main card order instead of creation order",
);
assert(
  creationOrder.map((account) => account.id).join(",") === "created-first,created-second,created-third",
  "sorting account cards never mutates the account store order",
);
