import data from "./catalog.json";
import type { PetItem, PetSlot } from "./types";
export const PET_ITEMS = data as PetItem[];
export const PET_ITEM_BY_ID = new Map(PET_ITEMS.map(item => [item.id, item]));
export const PET_SLOTS: { id: PetSlot; name: string; en: string }[] = [
  { id: "head", name: "头饰", en: "Head" }, { id: "face", name: "面部", en: "Face" },
  { id: "neck", name: "颈部", en: "Neck" }, { id: "body", name: "身体", en: "Body" },
  { id: "hands", name: "手部", en: "Paws" }, { id: "wings", name: "翅膀", en: "Wings" },
  { id: "aura", name: "光环", en: "Aura" }, { id: "companion", name: "身边", en: "Companion" },
];

/** Ground effects sit below wings, and capes cover the wing roots. */
export const PET_BACK_SLOT_ORDER: readonly PetSlot[] = ["aura", "wings", "body"];
