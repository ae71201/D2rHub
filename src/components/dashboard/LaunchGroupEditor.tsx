import { Check, ListChecks, Trash2 } from "lucide-react";
import type { LaunchGroupController } from "../../hooks/useLaunchGroupController";
import { useGlobalConfig } from "../../store/globalConfig";
export function LaunchGroupEditor({ launchGroups }: { launchGroups: LaunchGroupController }) {
  const saving = useGlobalConfig(state => state.saving);
  const draft = launchGroups.draft;
  const draftReady = Boolean(
    draft?.members.length
    && draft.name.trim()
    && draft.members.every(member =>
      member.graphics_configured && member.resolution && member.fps != null),
  );

  if (!draft) return null;
  return <div className="launch-group-inline-editor" aria-label="编辑启动方案">

        <>
          <div className="launch-group-editor flex min-w-0 items-center gap-2">
            <span className="launch-group-editor-label">
              <ListChecks size={13} strokeWidth={1.9} aria-hidden="true" />
              {draft.id ? "编辑启动方案" : "新建启动方案"}
            </span>
            <input
              type="text"
              className="line-input launch-group-name-input px-2.5"
              value={draft.name}
              maxLength={32}
              aria-label="启动方案名称"
              placeholder="启动方案名称"
              autoFocus
              disabled={saving}
              onChange={event => launchGroups.renameDraft(event.target.value)}
              onKeyDown={event => {
                if (event.key === "Enter") void launchGroups.saveDraft();
                if (event.key === "Escape") launchGroups.cancelDraft();
              }}
            />
            <button
              disabled={saving || !draftReady}
              onClick={() => void launchGroups.saveDraft()}
              className="primary-cta"
            >
              <Check size={13} strokeWidth={2} />
              保存方案 ({draft.members.length})
            </button>
            <button onClick={launchGroups.selectAll} disabled={saving} className="control-btn">
              全选
            </button>
            <button
              onClick={launchGroups.clearSelection}
              disabled={saving || draft.members.length === 0}
              className="control-btn"
            >
              清空已选
            </button>
            <button onClick={launchGroups.cancelDraft} disabled={saving} className="control-btn">
              取消
            </button>
          </div>
          <div className="flex-1" />
          {draft.id && (
            <button
              type="button"
              className="control-btn danger-control"
              disabled={saving}
              onClick={launchGroups.requestDraftDelete}
            >
              <Trash2 size={12} strokeWidth={1.8} aria-hidden="true" />
              删除方案
            </button>
          )}
        </>

    <p className="scheme-editor-hint">选择下方账号，并调整各账号的方案配置。保存后生效，取消不会修改原方案。</p>
  </div>;
}
