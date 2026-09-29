import { PackageOpen, UserPlus } from "lucide-react";

import type { LaunchGroupController } from "../../hooks/useLaunchGroupController";
import { useAccounts } from "../../store/accounts";
import { useGlobalConfig } from "../../store/globalConfig";
import { ActionBar } from "./ActionBar";
import { FavoriteLaunchGroups } from "./FavoriteLaunchGroups";
import { LaunchButton } from "./LaunchButton";
import { LaunchGroupMenu } from "./LaunchGroupMenu";
import { RoomAutomationQuickEdit } from "./RoomAutomationQuickEdit";
import type { ModCapsulePool } from "../../store/types";
import type { BatchSelection } from "../../hooks/useAccountBatch";

interface MainActionBarProps {
  batchSelection?: BatchSelection;
  batchBusy?: boolean;
  batchUncertain?: boolean;
  onClearBatch?: () => void;
  launching: boolean;
  launchableAccountIds: string[];
  launchGroups: LaunchGroupController;
  onCancelLaunch: () => void;
  onStartLaunch: (accountIds: string[]) => void;
  onAddAccount: () => void;
  onRequestKillAll: () => void;
  launchGroupPanelOpen: boolean;
  onToggleLaunchGroupPanel: () => void;
  onOpenModManager: () => void;
  onOpenRoomAutomation: () => void;
  showOptionalFeatures?: boolean;
  modCapsulePool?: ModCapsulePool | null;
}

export function MainActionBar({
  batchSelection, batchBusy, batchUncertain, onClearBatch,
  launching,
  launchableAccountIds,
  launchGroups,
  onCancelLaunch,
  onStartLaunch,
  onAddAccount,
  onRequestKillAll,
  launchGroupPanelOpen,
  onToggleLaunchGroupPanel,
  onOpenModManager,
  onOpenRoomAutomation,
  showOptionalFeatures = true,
  modCapsulePool,
}: MainActionBarProps) {
  const { config, saving } = useGlobalConfig();
  const { accounts } = useAccounts();
  const draft = launchGroups.draft;

  return (
    <ActionBar disabled={!!draft}>
      {launching ? (
        <button onClick={onCancelLaunch} className="danger-cta">
          取消操作
        </button>
      ) : (
        <>
        <div className="flex min-w-0 items-center gap-2">
          <LaunchButton
            count={batchSelection?.mode === "launch" ? batchSelection.ids.length : launchableAccountIds.length}
            selected={batchSelection?.mode === "launch"}
            loading={launching || !!batchBusy || !!batchUncertain || saving || batchSelection?.mode === "close"}
            onClick={() => onStartLaunch(batchSelection?.mode === "launch" ? batchSelection.ids : launchableAccountIds)}
          />
          <FavoriteLaunchGroups
            groups={config?.launch_groups ?? []}
            favoriteGroupIds={config?.favorite_launch_group_ids}
            accounts={accounts}
            config={config}
            modCapsulePool={modCapsulePool}
            disabled={launching || saving || !!batchSelection?.mode || !!batchBusy}
            onLaunch={launchGroups.launch}
            onToggleFavorite={group => void launchGroups.toggleFavorite(group)}
          />
          <button
            onClick={onRequestKillAll}
            disabled={!!batchBusy || batchSelection?.mode === "launch" || (batchSelection?.mode === "close" && batchUncertain)}
            title={batchSelection?.mode === "close" ? "仅关闭选中账号的游戏进程" : "一键关闭所有暗黑2进程"}
            className="control-btn danger-control ml-1 min-w-[72px]"
          >
            {batchBusy ? "处理中…" : batchSelection?.mode === "close" ? `关闭选中 (${batchSelection.ids.length})` : "一键关闭"}
          </button>
          {batchSelection?.mode && <button type="button" className="control-btn" disabled={batchBusy} onClick={onClearBatch}>取消选择</button>}
          {batchUncertain && <span className="micro-meta" role="status">状态待确认</span>}
          {showOptionalFeatures && (
            <RoomAutomationQuickEdit
              active={config?.installed_optional_modules?.includes("room-automation") === true}
              language={config?.app_language}
              onOpenSettings={onOpenRoomAutomation}
            />
          )}
        </div>
        <div className="flex-1" />
        <button type="button" className="control-btn" onClick={onOpenModManager}>
          <PackageOpen size={13} strokeWidth={1.9} aria-hidden="true" />
          Mod 管理
        </button>
        <LaunchGroupMenu
          count={config?.launch_groups.length ?? 0}
          open={launchGroupPanelOpen}
          disabled={launching || saving || !!batchBusy}
          onToggle={onToggleLaunchGroupPanel}
        />
        </>
      )}
      <>
          <button onClick={onAddAccount} className="control-btn add-account-cta">
            <UserPlus size={13} strokeWidth={1.9} />
            添加账号
          </button>
      </>
    </ActionBar>
  );
}
