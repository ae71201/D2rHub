import { cleanup, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { invokeCommand } from "../../../platform/tauri";
import { ModWorkspace } from "./ModWorkspace";
import { modPool, testCatalog, workflowFixture } from "./testFixtures";

vi.mock("../../../platform/tauri", () => ({ invokeCommand: vi.fn() }));
vi.mock("../../../components/ui/Toast", () => ({ showToast: vi.fn() }));
vi.mock("../../tasks/taskSync", () => ({ subscribeBeforeReadingTasks: vi.fn(async () => () => {}) }));
beforeEach(() => vi.mocked(invokeCommand).mockReset());
afterEach(cleanup);

describe("Mod library processor navigation", () => {
  it("opens downloads for the selected edition and returns without assigning or starting a recipe", async () => {
    vi.mocked(invokeCommand).mockResolvedValue({ catalog: { assets: [] }, processor: { ready: false } } as never);
    const catalog = testCatalog({ ...modPool, capsules: [{
      id: "plain", edition: "Global", name: "Plain", origin: "scanned", source_eligible: true,
      launch_arguments: "-mod Plain -txt", default_launch_arguments: "-mod Plain -txt", feature_groups: [], assigned_account_ids: [], ready: true,
      processed: false, update_required: false, deletable: true,
    }] });
    const workflow = workflowFixture({ view: "library", libraryEdition: "Global", catalog });
    render(<ModWorkspace workflow={workflow} />);
    await userEvent.click(screen.getByRole("button", { name: "设置 Plain" }));
    await userEvent.click(screen.getByRole("button", { name: "加工功能" }));
    await screen.findByRole("heading", { name: "Mod 下载与更新" });
    expect(invokeCommand).toHaveBeenCalledWith("get_mod_resources", { edition: "Global", refresh: false });
    expect(workflow.actions.requestProcessing).not.toHaveBeenCalled();
    expect(workflow.actions.prepare).not.toHaveBeenCalled();
    expect(catalog.assign).not.toHaveBeenCalled();
    await userEvent.click(screen.getByRole("button", { name: "返回" }));
    expect(screen.getByRole("article", { name: "Plain" })).toBeTruthy();
  });
});
