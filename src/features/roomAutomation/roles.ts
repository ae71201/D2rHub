/** Swap only the old/new leader; preserve follower order and membership. */
export function switchPrimary<T extends { primary_account_id: string; follower_account_ids: string[] }>(current: T, next: string): T {
  const old = current.primary_account_id;
  if (old === next) return { ...current, follower_account_ids: [...current.follower_account_ids] };
  const followers = current.follower_account_ids.includes(next)
    ? current.follower_account_ids.map(id => id === next ? old : id)
    : [old, ...current.follower_account_ids];
  return { ...current, primary_account_id: next, follower_account_ids: [...new Set(followers.filter(id => id && id !== next))] };
}
