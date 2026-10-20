import { invokeCommand } from "../../platform/tauri";

export interface VisionResourcePackage {
  ready: boolean;
  preparing: boolean;
  progress: number | null;
  message: string;
  size: number;
  path: string;
}

export const visionResourcesGateway = {
  read: (refresh = false) => invokeCommand<VisionResourcePackage>("vision_get_resource_package", { refresh }),
  prepare: () => invokeCommand<VisionResourcePackage>("vision_prepare_resource_package"),
  openFolder: () => invokeCommand("vision_open_resource_directory"),
};
