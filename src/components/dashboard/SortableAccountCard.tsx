import { useSortable } from "@dnd-kit/sortable";
import { CSS } from "@dnd-kit/utilities";
import { GripVertical } from "lucide-react";

import { useLaunch } from "../../store/launch";
import { AccountGridItem, type GridItemProps } from "./AccountCard";

export function SortableAccountCard({
  account,
  isSelectionMode,
  ...cardProps
}: GridItemProps) {
  const sortingDisabled = !!isSelectionMode || !!cardProps.runtimeStatus?.disabled || !!cardProps.runtimeStatus?.mode;
  const english = cardProps.config?.app_language === "en-US";
  const {
    attributes,
    listeners,
    setNodeRef,
    setActivatorNodeRef,
    transform,
    transition,
    isDragging,
  } = useSortable({
    id: account.id,
    disabled: sortingDisabled,
  });
  const { progress } = useLaunch();

  return (
    <div
      ref={setNodeRef}
      data-sortable-account-id={account.id}
      style={{
        transform: CSS.Transform.toString(transform),
        transition,
        opacity: isDragging ? 0.5 : 1,
        zIndex: isDragging ? 10 : undefined,
        width: "100%",
      }}
    >
      <AccountGridItem
        {...cardProps}
        account={account}
        progress={progress[account.id] || null}
        isSelectionMode={isSelectionMode}
        dragHandle={!isSelectionMode && <button type="button" ref={setActivatorNodeRef}
          className="account-drag-handle" {...attributes} {...listeners}
          disabled={sortingDisabled}
          aria-label={`${english ? "Reorder" : "拖动排序"} ${account.display_name || account.id}`}
          title={english ? "Drag to reorder; or press Space, then use arrow keys" : "拖动排序；键盘按空格后使用方向键移动"}
          onClick={event => event.stopPropagation()}>
          <GripVertical size={12} aria-hidden="true" />
        </button>}
      />
    </div>
  );
}
