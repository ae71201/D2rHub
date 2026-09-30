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
  it("opens processing directly without a download or pairing request", async () => {
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
    expect(invokeCommand).not.toHaveBeenCalled();
    expect(workflow.actions.requestProcessing).toHaveBeenCalledWith({ origin: "library", edition: "Global", source: { name: "Plain", processed: false } });
    expect(catalog.assign).not.toHaveBeenCalled();
  });
});
