import { act, cleanup, fireEvent, render, screen, within } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { VisionResourceCard } from "./VisionResourceCard";
const gateway = vi.hoisted(() => ({ read: vi.fn(), prepare: vi.fn(), openFolder: vi.fn() }));
vi.mock("./visionResourcesGateway", () => ({ visionResourcesGateway: gateway }));
const status = (extra = {}) => ({ ready: false, preparing: false, progress: null,
  message: "尚未准备完整资源包，启用识别时会自动下载", size: 88999851, path: "C:\\resources\\vision", ...extra });
beforeEach(() => {
  vi.useFakeTimers();
  gateway.read.mockReset().mockResolvedValue(status());
  gateway.prepare.mockReset();
  gateway.openFolder.mockReset().mockResolvedValue(undefined);
});
afterEach(() => { cleanup(); vi.useRealTimers(); });
const mount = async () => { await act(async () => { render(<VisionResourceCard en={false} />); }); };

describe("shared visual resource package", () => {
  it("shows one package independently of game installation and opens its shared directory", async () => {
    await mount();
    expect(screen.getAllByRole("article")).toHaveLength(1);
    const card = screen.getByRole("article", { name: "视觉识别资源包" });
    expect(within(card).getByText("下载 84.9 MB")).toBeTruthy();
    expect(within(card).queryByText(/onnxruntime|PP-OCR/)).toBeNull();
    await act(async () => fireEvent.click(screen.getByRole("button", { name: "打开目录" })));
    expect(gateway.openFolder).toHaveBeenCalledOnce();
    expect(gateway.read).toHaveBeenCalledWith(true);
  });
  it("restores automatic preparation progress and refreshes when it completes", async () => {
    gateway.read.mockResolvedValue(status({ preparing: true, progress: 37, message: "下载视觉识别资源包：37%" }));
    await mount();
    expect(screen.getByRole("progressbar").getAttribute("aria-valuenow")).toBe("37");
    expect(screen.getByRole("button", { name: "正在准备…" })).toHaveProperty("disabled", true);
    gateway.read.mockResolvedValue(status({ ready: true, progress: 100, message: "资源包已就绪" }));
    await act(async () => vi.advanceTimersByTime(1500));
    expect(screen.getByRole("button", { name: "校验资源包" })).toHaveProperty("disabled", false);
    expect(screen.queryByRole("progressbar")).toBeNull();
  });
  it("prepares one package and prevents duplicate clicks while the command is pending", async () => {
    let finish!: (value: ReturnType<typeof status>) => void;
    gateway.prepare.mockImplementation(() => new Promise(resolve => { finish = resolve; }));
    await mount();
    await act(async () => fireEvent.click(screen.getByRole("button", { name: "下载并准备" })));
    fireEvent.click(screen.getByRole("button", { name: "正在准备…" }));
    expect(gateway.prepare).toHaveBeenCalledOnce();
    await act(async () => finish(status({ ready: true, progress: 100, message: "资源包已就绪" })));
    expect(screen.getByRole("button", { name: "校验资源包" })).toBeTruthy();
  });
  it("allows retry after a failed preparation", async () => {
    gateway.prepare.mockRejectedValueOnce(new Error("下载源不可用"))
      .mockResolvedValueOnce(status({ ready: true, message: "资源包已就绪" }));
    await mount();
    await act(async () => fireEvent.click(screen.getByRole("button", { name: "下载并准备" })));
    expect(screen.getByRole("alert").textContent).toContain("下载源不可用");
    await act(async () => vi.advanceTimersByTime(1500));
    await act(async () => fireEvent.click(screen.getByRole("button", { name: "下载并准备" })));
    expect(gateway.prepare).toHaveBeenCalledTimes(2);
    expect(screen.queryByRole("alert")).toBeNull();
  });
});
