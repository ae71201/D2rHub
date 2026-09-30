export interface ModResourceAsset {
  id: string;
  version: string;
  url: string;
  mirrors?: { platform: string; url: string }[];
  size: number;
  game_data_version: string | null;
}

export interface InstalledModResource {
  id: string;
  installed_version: string | null;
  update_available: boolean;
  protected: boolean;
  message: string;
  reason_code?: string | null;
  integrity_checked?: boolean;
}

export interface ModResourceState {
  catalog: { release_url: string; assets: ModResourceAsset[] };
  mods_directory: string | null;
  game_data_version: string | null;
  warning: string | null;
  preferred_source?: string;
  mods?: InstalledModResource[];
}

export interface ResourceFeedback {
  assetId: string | null;
  error?: string;
  installedPath?: string;
  installAfterTaskId?: number;
}
