import { create } from "zustand";
import {
  Hardware,
  HardwareInstructions,
  Index,
  ScanResult,
  Version,
  ipc,
  onDownloadProgress,
  onSdEvent,
} from "@/lib/ipc";

type View =
  | { kind: "home" }
  | { kind: "history"; hardware: Hardware }
  | { kind: "wizard"; hardware: Hardware; version: Version }
  | { kind: "settings" }
  | { kind: "about" };

interface DownloadState {
  status: "idle" | "downloading" | "verifying" | "done" | "failed";
  received: number;
  total: number | null;
  path?: string;
  error?: string;
}

interface AppState {
  view: View;
  setView(v: View): void;

  onboarded: boolean | null;
  finishOnboarding(): Promise<void>;

  index: Index | null;
  indexError: string | null;
  reloadIndex(): Promise<void>;

  sdMounts: string[];
  manualMounts: string[];
  activeSd: string | null;
  setActiveSd(path: string | null): void;
  addManualMount(path: string): void;
  sdScan: ScanResult | null;
  rescanSd(path: string | null): Promise<void>;

  instructions: Record<string, HardwareInstructions>;

  /** Per-filename download progress keyed by the `.img` filename. */
  downloads: Record<string, DownloadState>;

  downloadDir: string | null;
  reloadDownloadDir(): Promise<void>;

  refreshCached(): Promise<void>;

  init(): Promise<void>;
}

// Guard init() so React 18 StrictMode double-mount can't double-subscribe
// event listeners or flicker onboarding state.
let initPromise: Promise<void> | null = null;

export const useStore = create<AppState>((set, get) => ({
  view: { kind: "home" },
  setView: (v) => set({ view: v }),

  onboarded: null,
  async finishOnboarding() {
    await ipc.markOnboarded();
    set({ onboarded: true });
  },

  index: null,
  indexError: null,
  async reloadIndex() {
    try {
      const index = await ipc.fetchIndex();
      set({ index, indexError: null });
    } catch (e) {
      set({ indexError: String(e) });
    }
  },

  sdMounts: [],
  manualMounts: [],
  activeSd: null,
  setActiveSd(path) {
    set({ activeSd: path });
    void get().rescanSd(path);
  },
  addManualMount(path) {
    const { manualMounts, sdMounts } = get();
    const next = manualMounts.includes(path)
      ? manualMounts
      : [...manualMounts, path];
    set({
      manualMounts: next,
      sdMounts: sdMounts.includes(path) ? sdMounts : [...sdMounts, path],
      activeSd: path,
    });
    void get().rescanSd(path);
  },
  sdScan: null,
  async rescanSd(path) {
    if (!path) {
      set({ sdScan: null });
      return;
    }
    try {
      const scan = await ipc.scanSd(path);
      set({ sdScan: scan });
    } catch (e) {
      console.error("scan failed", e);
      set({ sdScan: null });
    }
  },

  instructions: {},

  downloads: {},

  downloadDir: null,
  async reloadDownloadDir() {
    try {
      const d = await ipc.getDownloadDir();
      set({ downloadDir: d });
    } catch (e) {
      console.error("getDownloadDir failed", e);
    }
    await get().refreshCached();
  },

  /** Populate `downloads` with status:"done" entries for anything already on
   * disk, so cards can show Reveal instead of Download without a per-card
   * round-trip. */
  async refreshCached() {
    try {
      const cached = await ipc.listCachedFirmware();
      const current = get().downloads;
      const next = { ...current };
      for (const [filename, path] of Object.entries(cached)) {
        const existing = next[filename];
        // Don't clobber an in-flight download.
        if (existing && (existing.status === "downloading" || existing.status === "verifying")) {
          continue;
        }
        next[filename] = {
          status: "done",
          received: 0,
          total: null,
          path,
        };
      }
      set({ downloads: next });
    } catch (e) {
      console.error("listCachedFirmware failed", e);
    }
  },

  async init() {
    if (initPromise) return initPromise;
    initPromise = (async () => {
      const [onboarded, instructions] = await Promise.all([
        ipc.isOnboarded(),
        ipc.loadInstructions().catch(() => ({})),
      ]);
      set({ onboarded, instructions });

      await Promise.all([
        get().reloadIndex(),
        get().reloadDownloadDir(),
        get().refreshCached(),
      ]);

      onSdEvent((ev) => {
        const { sdMounts, activeSd, manualMounts } = get();
        if (ev.type === "mounted") {
          const mounts = sdMounts.includes(ev.path)
            ? sdMounts
            : [...sdMounts, ev.path];
          // Auto-pick the first mount, but let the user re-pick in the UI.
          const active = activeSd ?? ev.path;
          set({ sdMounts: mounts, activeSd: active });
          if (active === ev.path) {
            void get().rescanSd(ev.path);
          }
        } else {
          // Keep manually-added mounts even if the watcher says they're
          // gone — the user explicitly picked them.
          const remaining = sdMounts.filter(
            (p) => p !== ev.path || manualMounts.includes(p),
          );
          const nextActive =
            activeSd === ev.path && !manualMounts.includes(ev.path)
              ? (remaining[0] ?? null)
              : activeSd;
          set({ sdMounts: remaining, activeSd: nextActive });
          if (remaining.length === 0) {
            set({ sdScan: null });
          } else if (nextActive !== activeSd) {
            void get().rescanSd(nextActive);
          }
        }
      });

      onDownloadProgress((p) => {
        const cur = get().downloads;
        const update = (patch: DownloadState) =>
          set({ downloads: { ...cur, [p.filename]: patch } });
        const prev = cur[p.filename];
        switch (p.kind) {
          case "started":
            update({ status: "downloading", received: 0, total: null });
            break;
          case "progress":
            update({
              status: "downloading",
              received: p.received,
              total: p.total,
            });
            break;
          case "verifying":
            update({
              status: "verifying",
              received: prev?.received ?? 0,
              total: prev?.total ?? null,
            });
            break;
          case "done":
            update({
              status: "done",
              received: prev?.received ?? 0,
              total: prev?.total ?? null,
              path: p.path,
            });
            break;
          case "failed":
            update({
              status: "failed",
              received: prev?.received ?? 0,
              total: prev?.total ?? null,
              error: p.reason,
            });
            break;
        }
      });
    })();
    return initPromise;
  },
}));
