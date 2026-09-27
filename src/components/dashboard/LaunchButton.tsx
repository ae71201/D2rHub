
import { Zap } from "lucide-react";

export function LaunchButton({ count, loading, onClick, selected = false }: {
  count: number; loading: boolean; onClick: () => void; selected?: boolean;
}) {
  return (
    <button
      disabled={loading || count === 0}
      onClick={onClick}
      className="primary-cta"
    >
      <Zap size={13} strokeWidth={2} />
      {selected ? "启动选中" : "启动全部"} ({count})
    </button>
  );
}
