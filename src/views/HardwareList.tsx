import { useMemo } from "react";
import {
  FirmwareRelease,
  HARDWARE_LABELS,
  HARDWARE_LIST,
  Hardware,
  ipc,
  versionString,
} from "@/lib/ipc";
import { useStore } from "@/state/store";
import { DownloadIcon, HistoryIcon, RevealIcon, StageIcon } from "@/components/Icons";
import IconButton from "@/components/IconButton";
import ProgressBar from "@/components/ProgressBar";
import HardwareIcon from "@/components/HardwareIcon";
import ChannelBadge from "@/components/ChannelBadge";

export default function HardwareList() {
  const index = useStore((s) => s.index);
  const indexError = useStore((s) => s.indexError);
  const setView = useStore((s) => s.setView);

  const latestByHardware = useMemo(() => {
    const m: Record<string, FirmwareRelease | undefined> = {};
    if (!index) return m;
    for (const hw of HARDWARE_LIST) {
      m[hw] = [...index.releases]
        .filter((r) => r.channel === "stable" && r.downloads.some((d) => d.hardware === hw))
        .sort((a, b) =>
          b.version.major - a.version.major ||
          b.version.minor - a.version.minor ||
          b.version.patch - a.version.patch,
        )[0];
    }
    return m;
  }, [index]);

  if (indexError && !index) {
    return (
      <div className="rounded-lg border border-brand-danger/40 bg-brand-danger/10 p-4 text-sm">
        Couldn't reach any firmware source. Check your connection, then retry
        from Settings.
        <div className="mt-2 font-mono text-xs text-brand-dim">{indexError}</div>
      </div>
    );
  }

  return (
    <section className="space-y-4">
      <header className="flex items-baseline justify-between">
        <h2 className="text-2xl font-semibold">Firmware</h2>
        <IndexMeta />
      </header>
      <div className="grid grid-cols-1 gap-4 md:grid-cols-2 xl:grid-cols-3">
        {HARDWARE_LIST.map((hw) => (
          <HardwareCard
            key={hw}
            hardware={hw}
            release={latestByHardware[hw]}
            onHistory={() => setView({ kind: "history", hardware: hw })}
            onStage={(r) => setView({ kind: "wizard", hardware: hw, version: r.version })}
          />
        ))}
      </div>
    </section>
  );
}

function IndexMeta() {
  const index = useStore((s) => s.index);
  if (!index) return <span className="text-xs text-brand-muted">Loading…</span>;
  const label = SOURCE_LABELS[index.source];
  const href = SOURCE_URLS[index.source];
  return (
    <span className="text-xs text-brand-muted">
      Source:{" "}
      {href ? (
        <button
          onClick={() => ipc.openUrl(href)}
          className="underline hover:text-brand-fg"
        >
          {label}
        </button>
      ) : (
        label
      )}
      {" · fetched "}
      {new Date(index.fetched_at).toLocaleString()}
    </span>
  );
}

const SOURCE_LABELS: Record<string, string> = {
  walksnail_app: "walksnail.app",
  d3vl: "D3VL firmwares.json",
  official: "CADDXFPV Download Center",
  cache: "local cache",
};
const SOURCE_URLS: Record<string, string | undefined> = {
  walksnail_app: "https://walksnail.app/firmware",
  d3vl: "https://github.com/D3VL/Avatar-Firmware-Updates",
  official: "https://www.caddxfpv.com/pages/download-center",
  cache: undefined,
};

function HardwareCard(props: {
  hardware: Hardware;
  release?: FirmwareRelease;
  onHistory: () => void;
  onStage: (r: FirmwareRelease) => void;
}) {
  const { hardware, release, onHistory, onStage } = props;
  const sdScan = useStore((s) => s.sdScan);
  const matchesSd = sdScan?.contents.variant === hardware;
  const download = useStore((s) =>
    release ? s.downloads[release.downloads.find((d) => d.hardware === hardware)?.filename ?? ""] : undefined,
  );
  const isDownloading =
    download?.status === "downloading" || download?.status === "verifying";
  const isCached = download?.status === "done" && !!download.path;

  async function startDownload() {
    if (!release) return;
    try {
      await ipc.downloadFirmware(hardware, release.version);
    } catch {
      // failure is surfaced via the download-progress stream; nothing to do.
    }
  }

  return (
    <article
      className={`rounded-lg border p-4 ${
        matchesSd ? "border-brand-accent" : "border-brand-muted/30"
      } bg-white/5`}
    >
      <div className="flex items-start gap-3">
        <HardwareIcon
          hardware={hardware}
          className="h-16 w-16 shrink-0 text-brand-fg/80"
        />
        <div className="min-w-0 flex-1">
          <h3 className="text-lg font-medium leading-tight">
            {HARDWARE_LABELS[hardware]}
          </h3>
          {matchesSd && (
            <p className="mt-0.5 text-xs text-brand-accent">
              SD card matches this device
            </p>
          )}
          {release && (
            <>
              <div className="mt-1 flex flex-wrap items-center gap-2">
                <span className="font-mono text-lg">
                  {versionString(release.version)}
                </span>
                <ChannelBadge channel={release.channel} />
                {isCached && (
                  <span className="rounded bg-brand-info/20 px-1.5 py-0.5 text-[10px] uppercase tracking-wide text-brand-info">
                    Downloaded
                  </span>
                )}
              </div>
              <div className="text-xs text-brand-muted">
                {release.date ?? "—"}
              </div>
            </>
          )}
        </div>
        <div className="flex shrink-0 flex-col items-end gap-2">
          <IconButton label="View history" variant="ghost" onClick={onHistory}>
            <HistoryIcon className="h-4 w-4" />
          </IconButton>
          {release && (
            <div className="flex gap-2">
              {isCached ? (
                <IconButton
                  label="Reveal in file manager"
                  title="Already downloaded — reveal in Finder / Explorer"
                  variant="info"
                  onClick={() => ipc.revealInFileManager(download!.path!)}
                >
                  <RevealIcon className="h-4 w-4" />
                </IconButton>
              ) : (
                <IconButton
                  label="Download .img"
                  title={isDownloading ? "Downloading…" : "Download .img"}
                  variant="success"
                  disabled={isDownloading}
                  onClick={startDownload}
                >
                  <DownloadIcon className="h-4 w-4" />
                </IconButton>
              )}
              {matchesSd && (
                <IconButton
                  label="Stage to SD"
                  title="Stage this firmware to the detected SD card"
                  variant="primary"
                  onClick={() => onStage(release)}
                >
                  <StageIcon className="h-4 w-4" />
                </IconButton>
              )}
            </div>
          )}
        </div>
      </div>
      {release ? (
        <>
          {release.notes && (
            <p className="mt-3 line-clamp-3 whitespace-pre-wrap text-sm text-brand-dim">
              {release.notes}
            </p>
          )}
          {isDownloading && (
            <div className="mt-3">
              <ProgressBar
                received={download!.received}
                total={download!.total}
                verifying={download!.status === "verifying"}
              />
            </div>
          )}
          {download?.status === "failed" && (
            <p className="mt-2 text-xs text-brand-danger">{download.error}</p>
          )}
        </>
      ) : (
        <p className="mt-3 text-sm text-brand-muted">No release available yet.</p>
      )}
    </article>
  );
}
