import { invoke } from "@tauri-apps/api/core";
import { listen, UnlistenFn } from "@tauri-apps/api/event";

export type Hardware =
  | "AvatarSky"
  | "AvatarMiniSky"
  | "MoonlightSky"
  | "AvatarGnd"
  | "GogglesX"
  | "GogglesL"
  | "VrxSE"
  | "ReconHd"
  | "AvatarRelay";

export interface Version {
  major: number;
  minor: number;
  patch: number;
}

export function versionString(v: Version): string {
  return `${v.major}.${v.minor}.${v.patch}`;
}

export function parseVersion(s: string): Version | null {
  const m = /^(\d+)\.(\d+)\.(\d+)$/.exec(s);
  if (!m) return null;
  return { major: +m[1], minor: +m[2], patch: +m[3] };
}

export function cmpVersion(a: Version, b: Version): number {
  return a.major - b.major || a.minor - b.minor || a.patch - b.patch;
}

export type Channel = "stable" | "beta" | "unknown";

export interface Download {
  hardware: Hardware;
  filename: string;
  url: string;
  sha1: string;
}

export interface FirmwareRelease {
  version: Version;
  date: string | null;
  channel: Channel;
  notes: string;
  downloads: Download[];
}

export type SourceLabel = "walksnail_app" | "d3vl" | "official" | "cache";

export interface Index {
  releases: FirmwareRelease[];
  source: SourceLabel;
  fetched_at: string;
}

export type Signal =
  | { kind: "staged_img"; filename: string }
  | { kind: "srt_firmware"; version: Version; path: string }
  | { kind: "marker_file"; name: string }
  | { kind: "userfont_dir" }
  | { kind: "independ_upgrade" };

export interface SdContents {
  root: string;
  is_walksnail: boolean;
  variant: Hardware | null;
  staged_version: Version | null;
  running_version: Version | null;
  signals: Signal[];
}

export type SafetyVerdict =
  | { kind: "ok" }
  | { kind: "warn"; reason: string }
  | { kind: "block"; reason: string };

export interface ScanResult {
  contents: SdContents;
  latest_stable: FirmwareRelease | null;
  verdict: SafetyVerdict | null;
}

export type SdEvent =
  | { type: "mounted"; path: string }
  | { type: "removed"; path: string };

export type StageProgress =
  | { kind: "downloading"; received: number; total: number | null }
  | { kind: "verifying" }
  | { kind: "copying" }
  | {
      kind: "done";
      written: string;
      deleted_previous: string[];
      wrote_independ_upgrade: boolean;
    };

export type DownloadProgress =
  | { kind: "started"; filename: string }
  | {
      kind: "progress";
      filename: string;
      received: number;
      total: number | null;
    }
  | { kind: "verifying"; filename: string }
  | {
      kind: "done";
      filename: string;
      path: string;
      sha_verified: boolean;
    }
  | { kind: "failed"; filename: string; reason: string };

export const ipc = {
  fetchIndex(
    source?: "auto" | "walksnail_app_only" | "d3vl_only" | "official_only",
  ): Promise<Index> {
    return invoke("fetch_index", { source });
  },
  listReleases(hardware: Hardware): Promise<FirmwareRelease[]> {
    return invoke("list_releases", { hardware });
  },
  latestFor(hardware: Hardware): Promise<FirmwareRelease | null> {
    return invoke("latest_for", { hardware });
  },
  scanSd(path: string): Promise<ScanResult> {
    return invoke("scan_sd", { path });
  },
  downloadFirmware(hardware: Hardware, version: Version): Promise<string> {
    return invoke("download_firmware", { args: { hardware, version } });
  },
  stageFirmware(
    hardware: Hardware,
    version: Version,
    sdRoot: string,
    moonlightDvrOnly = false,
  ): Promise<unknown> {
    return invoke("stage_firmware", {
      args: {
        hardware,
        version,
        sd_root: sdRoot,
        moonlight_dvr_only: moonlightDvrOnly,
      },
    });
  },
  revealInFileManager(path: string): Promise<void> {
    return invoke("reveal_in_file_manager", { path });
  },
  openUrl(url: string): Promise<void> {
    return invoke("open_url", { url });
  },
  loadInstructions(): Promise<Record<string, HardwareInstructions>> {
    return invoke<{ hardware: Record<string, HardwareInstructions> }>(
      "load_instructions",
    ).then((r) => r.hardware);
  },
  getAppPaths(): Promise<{ data_dir: string; cache_dir: string }> {
    return invoke("get_app_paths");
  },
  getDownloadDir(): Promise<string> {
    return invoke("get_download_dir");
  },
  setDownloadDir(path: string | null): Promise<string> {
    return invoke("set_download_dir", { args: { path } });
  },
  getDownloadDirPref(): Promise<string | null> {
    return invoke("get_download_dir_pref");
  },
  listCachedFirmware(): Promise<Record<string, string>> {
    return invoke("list_cached_firmware");
  },
  markOnboarded(): Promise<void> {
    return invoke("mark_onboarded");
  },
  isOnboarded(): Promise<boolean> {
    return invoke("is_onboarded");
  },
};

export interface HardwareInstructions {
  steps: string[];
  source_url: string;
  fetched_at: string;
  manual_sha256: string;
}

export async function onSdEvent(
  handler: (ev: SdEvent) => void,
): Promise<UnlistenFn> {
  const mounted = await listen<SdEvent>("sd:mounted", (e) => handler(e.payload));
  const removed = await listen<SdEvent>("sd:removed", (e) => handler(e.payload));
  return () => {
    mounted();
    removed();
  };
}

export function onStageProgress(
  handler: (p: StageProgress) => void,
): Promise<UnlistenFn> {
  return listen<StageProgress>("stage:progress", (e) => handler(e.payload));
}

export function onDownloadProgress(
  handler: (p: DownloadProgress) => void,
): Promise<UnlistenFn> {
  return listen<DownloadProgress>("download:progress", (e) => handler(e.payload));
}

export const HARDWARE_LIST: Hardware[] = [
  "AvatarSky",
  "AvatarMiniSky",
  "MoonlightSky",
  "AvatarGnd",
  "GogglesX",
  "GogglesL",
  "VrxSE",
  "ReconHd",
  "AvatarRelay",
];

export const HARDWARE_LABELS: Record<Hardware, string> = {
  AvatarSky: "Avatar HD VTX",
  AvatarMiniSky: "Avatar Mini 1S VTX",
  MoonlightSky: "Moonlight 4K VTX",
  AvatarGnd: "Avatar HD Goggles",
  GogglesX: "Goggles X",
  GogglesL: "Goggles L",
  VrxSE: "Avatar VRX",
  ReconHd: "Recon HD",
  AvatarRelay: "Avatar Relay",
};
