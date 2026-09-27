import { act, cleanup, renderHook, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { invokeCommand } from "../../platform/tauri";
import { useApplicationDisclosure } from "./useApplicationDisclosure";
import { APPLICATION_DISCLOSURE_REVISION, hasAcceptedApplicationDisclosure } from "./disclosureStorage";

vi.mock("../../platform/tauri", () => ({ invokeCommand: vi.fn() }));
vi.mock("../../store/accounts", () => ({ useAccounts: { getState: () => ({ loadAccounts: async () => {} }) } }));

let receipt: number | null;
let version: string;
let storageFailure: boolean;
let runtimeFailure: boolean;
let readFailure: boolean;

beforeEach(() => {
  localStorage.clear();
  receipt = null;
  version = "0.9.103";
  storageFailure = false;
  runtimeFailure = false;
  readFailure = false;
  vi.mocked(invokeCommand).mockImplementation(async command => {
    if (command === "get_app_version") return version;
    if (command === "get_application_disclosure_acceptance") {
      if (readFailure) throw new Error("read failed");
      return receipt;
    }
    if (command === "accept_application_disclosure") {
      if (storageFailure) throw new Error("write failed");
      receipt = APPLICATION_DISCLOSURE_REVISION;
      return;
    }
    if (command === "activate_application_runtime") {
      if (runtimeFailure) throw new Error("startup failed");
      return true;
    }
    throw new Error(`Unexpected command ${command}`);
  });
});
afterEach(cleanup);

describe("durable application notice acceptance", () => {
  it("migrates a 102 receipt on 103 and survives WebView storage clearing and app upgrades", async () => {
    localStorage.setItem("d2rhub-disclosure-accepted-version", "0.9.102");
    const first = renderHook(() => useApplicationDisclosure(true));
    await waitFor(() => expect(first.result.current.checking).toBe(false));
    expect(first.result.current.required).toBe(false);
    expect(receipt).toBe(APPLICATION_DISCLOSURE_REVISION);
    first.unmount();
    localStorage.clear();
    version = "0.9.104";
    const next = renderHook(() => useApplicationDisclosure(true));
    await waitFor(() => expect(next.result.current.checking).toBe(false));
    expect(next.result.current.required).toBe(false);
    expect(next.result.current.error).toBeNull();
  });

  it("keeps consent when services fail, then retries without asking again", async () => {
    receipt = APPLICATION_DISCLOSURE_REVISION;
    runtimeFailure = true;
    const { result } = renderHook(() => useApplicationDisclosure(true));
    await waitFor(() => expect(result.current.error?.stage).toBe("runtime"));
    expect(result.current.required).toBe(false);
    runtimeFailure = false;
    act(() => result.current.retry());
    await waitFor(() => expect(result.current.checking).toBe(false));
    expect(result.current.error).toBeNull();
    expect(invokeCommand).not.toHaveBeenCalledWith("accept_application_disclosure", expect.anything());
  });

  it("does not release startup when saving acceptance fails", async () => {
    const { result } = renderHook(() => useApplicationDisclosure(true));
    await waitFor(() => expect(result.current.required).toBe(true));
    storageFailure = true;
    await act(async () => { await expect(result.current.accept()).rejects.toThrow("write failed"); });
    expect(result.current.required).toBe(true);
    expect(receipt).toBeNull();
    expect(invokeCommand).not.toHaveBeenCalledWith("activate_application_runtime");
    storageFailure = false;
    await act(async () => { await result.current.accept(); });
    await waitFor(() => expect(result.current.checking).toBe(false));
    expect(result.current.required).toBe(false);
    expect(receipt).toBe(APPLICATION_DISCLOSURE_REVISION);
  });

  it("persists first acceptance before a failed runtime start and retains it on reopening", async () => {
    runtimeFailure = true;
    const first = renderHook(() => useApplicationDisclosure(true));
    await waitFor(() => expect(first.result.current.required).toBe(true));
    await act(async () => { await first.result.current.accept(); });
    await waitFor(() => expect(first.result.current.error?.stage).toBe("runtime"));
    expect(first.result.current.required).toBe(false);
    expect(receipt).toBe(APPLICATION_DISCLOSURE_REVISION);
    first.unmount();
    runtimeFailure = false;
    const next = renderHook(() => useApplicationDisclosure(true));
    await waitFor(() => expect(next.result.current.checking).toBe(false));
    expect(next.result.current.required).toBe(false);
  });

  it("shows a storage error for a failed migration, retains legacy data and allows retry", async () => {
    localStorage.setItem("d2rhub-disclosure-accepted-version", "0.9.102");
    storageFailure = true;
    const { result } = renderHook(() => useApplicationDisclosure(true));
    await waitFor(() => expect(result.current.error?.stage).toBe("storage"));
    expect(result.current.required).toBe(false);
    expect(localStorage.getItem("d2rhub-disclosure-accepted-version")).toBe("0.9.102");
    storageFailure = false;
    act(() => result.current.retry());
    await waitFor(() => expect(result.current.checking).toBe(false));
    expect(receipt).toBe(APPLICATION_DISCLOSURE_REVISION);
    expect(result.current.error).toBeNull();
  });

  it("does not mistake backend read errors for missing consent", async () => {
    readFailure = true;
    const { result } = renderHook(() => useApplicationDisclosure(true));
    await waitFor(() => expect(result.current.error?.stage).toBe("storage"));
    expect(result.current.required).toBe(false);
    expect(invokeCommand).not.toHaveBeenCalledWith("activate_application_runtime");
  });

  it("does not import invalid receipts or overwrite another notice revision", async () => {
    localStorage.setItem("d2rhub-disclosure-accepted-version", "unknown");
    expect(await hasAcceptedApplicationDisclosure()).toBe(false);
    receipt = APPLICATION_DISCLOSURE_REVISION + 1;
    localStorage.setItem("d2rhub-disclosure-accepted-version", "0.9.102");
    expect(await hasAcceptedApplicationDisclosure()).toBe(false);
    expect(receipt).toBe(APPLICATION_DISCLOSURE_REVISION + 1);
  });

  it("saves consent before profile selection and activates only after selection", async () => {
    const { result, rerender } = renderHook(({ runtimeReady }) => useApplicationDisclosure(true, runtimeReady), {
      initialProps: { runtimeReady: false },
    });
    await waitFor(() => expect(result.current.required).toBe(true));
    await act(async () => { await result.current.accept(); });
    await waitFor(() => expect(result.current.checking).toBe(false));
    expect(invokeCommand).not.toHaveBeenCalledWith("activate_application_runtime");
    rerender({ runtimeReady: true });
    await waitFor(() => expect(invokeCommand).toHaveBeenCalledWith("activate_application_runtime"));
    await waitFor(() => expect(result.current.checking).toBe(false));
    expect(result.current.required).toBe(false);
  });

  it("does not depend on obtaining an application version or writable localStorage", async () => {
    receipt = APPLICATION_DISCLOSURE_REVISION;
    vi.spyOn(Storage.prototype, "getItem").mockImplementation(() => { throw new Error("unavailable"); });
    vi.mocked(invokeCommand).mockImplementationOnce(async () => { throw new Error("version unavailable"); });
    const { result } = renderHook(() => useApplicationDisclosure(true));
    await waitFor(() => expect(result.current.checking).toBe(false));
    expect(result.current.version).toBe("unknown");
    expect(result.current.required).toBe(false);
    expect(result.current.error).toBeNull();
  });
});
