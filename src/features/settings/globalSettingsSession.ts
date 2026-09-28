import type { GlobalConfig } from "../../store/types";
import { diffGlobalConfig, type GlobalConfigPatch } from "../../utils/globalConfigPatch";

type ConfigKey = keyof GlobalConfig;
type FieldEdit = { value: unknown; revision: number };

/** A settings window owns edits; the global store only owns committed snapshots. */
export class GlobalSettingsSession {
  private baseline: GlobalConfig | null;
  private edits = new Map<ConfigKey, FieldEdit>();
  private pending = new Map<ConfigKey, number>();
  private revision = 0;

  constructor(config: GlobalConfig | null) {
    this.baseline = config;
  }

  rebase(config: GlobalConfig | null): void {
    if (!config) return;
    this.baseline = config;
    for (const [key, edit] of this.edits) {
      if (!this.pending.has(key) && JSON.stringify(edit.value) === JSON.stringify(config[key])) {
        this.edits.delete(key);
      }
    }
  }

  snapshot(): GlobalConfig | null {
    if (!this.baseline) return null;
    const next = { ...this.baseline };
    for (const [key, edit] of this.edits) {
      (next as unknown as Record<string, unknown>)[key] = edit.value;
    }
    return next;
  }

  private apply(patch: GlobalConfigPatch): void {
    for (const key of Object.keys(patch) as ConfigKey[]) {
      this.edits.set(key, { value: structuredClone(patch[key]), revision: ++this.revision });
    }
  }

  update(updater: (config: GlobalConfig) => void): void {
    const current = this.snapshot();
    if (!current) return;
    const next = structuredClone(current);
    updater(next);
    this.apply(diffGlobalConfig(current, next));
  }

  patch(): GlobalConfigPatch {
    const draft = this.snapshot();
    return draft ? diffGlobalConfig(this.baseline, draft) : {};
  }

  async save(
    renderedDraft: GlobalConfig | null,
    candidate: GlobalConfig | undefined,
    persist: (patch: GlobalConfigPatch) => Promise<GlobalConfig>,
    readCommitted: () => GlobalConfig | null,
  ): Promise<GlobalConfig> {
    this.rebase(readCommitted());
    // A caller supplies an intent relative to the draft it rendered. This
    // prevents a stale async action from writing unrelated old fields back.
    if (candidate && renderedDraft) this.apply(diffGlobalConfig(renderedDraft, candidate));
    const patch = this.patch();
    const sent = new Map<ConfigKey, number>();
    for (const key of Object.keys(patch) as ConfigKey[]) {
      sent.set(key, this.edits.get(key)!.revision);
      this.pending.set(key, (this.pending.get(key) ?? 0) + 1);
    }
    try {
      const saved = await persist(patch);
      for (const [key, revision] of sent) {
        // Typing while the request was in flight is a newer intent.
        if (this.edits.get(key)?.revision === revision) this.edits.delete(key);
      }
      return saved;
    } finally {
      for (const key of sent.keys()) {
        const remaining = (this.pending.get(key) ?? 1) - 1;
        if (remaining) this.pending.set(key, remaining);
        else this.pending.delete(key);
      }
      this.rebase(readCommitted());
    }
  }
}
