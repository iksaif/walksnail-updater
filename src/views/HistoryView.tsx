import { useMemo, useState } from "react";
import {
  FirmwareRelease,
  HARDWARE_LABELS,
  Hardware,
  cmpVersion,
  ipc,
  versionString,
} from "@/lib/ipc";
import { useStore } from "@/state/store";
import { DownloadIcon, RevealIcon, StageIcon } from "@/components/Icons";
import IconButton from "@/components/IconButton";
import HardwareIcon from "@/components/HardwareIcon";
import ProgressBar from "@/components/ProgressBar";
import ChannelBadge from "@/components/ChannelBadge";

export default function HistoryView({ hardware }: { hardware: Hardware }) {
  const index = useStore((s) => s.index);
  const setView = useStore((s) => s.setView);
  const sdScan = useStore((s) => s.sdScan);

  const releases = useMemo(() => {
    if (!index) return [] as FirmwareRelease[];
    return [...index.releases]
      .filter((r) => r.downloads.some((d) => d.hardware === hardware))
      .sort((a, b) => cmpVersion(b.version, a.version));
  }, [index, hardware]);

  const currentVersion =
    (sdScan?.contents.variant === hardware &&
      (sdScan.contents.running_version ?? sdScan.contents.staged_version)) ||
    null;

  return (
    <section className="space-y-4">
      <header className="flex items-center gap-3">
        <HardwareIcon hardware={hardware} className="h-14 w-14 text-brand-fg/80" />
        <div>
          <button
            onClick={() => setView({ kind: "home" })}
            className="text-xs text-brand-muted hover:text-brand-fg"
          >
            ← Back
          </button>
          <h2 className="text-2xl font-semibold leading-tight">
            {HARDWARE_LABELS[hardware]} · history
          </h2>
        </div>
        <span className="ml-auto text-xs text-brand-muted">
          {releases.length} release{releases.length === 1 ? "" : "s"}
        </span>
      </header>
      <ul className="space-y-2">
        {releases.map((r, idx) => (
          <VersionRow
            key={versionString(r.version)}
            release={r}
            hardware={hardware}
            current={currentVersion}
            canStage={sdScan?.contents.variant === hardware}
            defaultOpen={idx === 0}
          />
        ))}
      </ul>
    </section>
  );
}

/// Max characters shown in the collapsed changelog view. Roughly matches the
/// 3 visible lines requirement at typical card widths without counting DOM
/// lines explicitly.
const COLLAPSED_NOTE_CHARS = 240;

function VersionRow(props: {
  release: FirmwareRelease;
  hardware: Hardware;
  current: { major: number; minor: number; patch: number } | null;
  canStage: boolean;
  defaultOpen: boolean;
}) {
  const { release, hardware, current, canStage, defaultOpen } = props;
  const setView = useStore((s) => s.setView);
  const download = useStore(
    (s) =>
      s.downloads[
        release.downloads.find((d) => d.hardware === hardware)?.filename ?? ""
      ],
  );
  const [expanded, setExpanded] = useState(defaultOpen);
  const isCurrent =
    current !== null && cmpVersion(current, release.version) === 0;
  const isDownloading =
    download?.status === "downloading" || download?.status === "verifying";
  const isCached = download?.status === "done" && !!download.path;

  const notes = (release.notes ?? "").trim();
  const truncated =
    notes.length > COLLAPSED_NOTE_CHARS &&
    !defaultOpen &&
    !expanded;
  const shownNotes = truncated
    ? `${notes.slice(0, COLLAPSED_NOTE_CHARS).trimEnd()}…`
    : notes;
  const canToggle = notes.length > COLLAPSED_NOTE_CHARS && !defaultOpen;

  return (
    <li
      className={`rounded border p-3 ${
        isCurrent ? "border-brand-accent" : "border-brand-muted/20"
      }`}
    >
      <div className="flex items-center justify-between gap-3">
        <div className="flex flex-wrap items-center gap-x-3 gap-y-1">
          <span className="font-mono text-lg">{versionString(release.version)}</span>
          <span className="text-xs text-brand-muted">{release.date ?? "—"}</span>
          <ChannelBadge channel={release.channel} />
          {isCurrent && (
            <span className="rounded bg-brand-accent/20 px-1.5 text-xs text-brand-accent">
              Current
            </span>
          )}
          {isCached && (
            <span className="rounded bg-brand-info/20 px-1.5 text-[10px] uppercase text-brand-info">
              Downloaded
            </span>
          )}
        </div>
        <div className="flex items-center gap-2">
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
              label="Download"
              variant="success"
              disabled={isDownloading}
              onClick={() =>
                ipc.downloadFirmware(hardware, release.version).catch(() => {})
              }
            >
              <DownloadIcon className="h-4 w-4" />
            </IconButton>
          )}
          {canStage && (
            <IconButton
              label="Stage to SD"
              variant="primary"
              onClick={() =>
                setView({ kind: "wizard", hardware, version: release.version })
              }
            >
              <StageIcon className="h-4 w-4" />
            </IconButton>
          )}
        </div>
      </div>
      {notes ? (
        <div className="mt-2 text-xs text-brand-dim">
          <p className="whitespace-pre-wrap">{shownNotes || "—"}</p>
          {canToggle && (
            <button
              onClick={() => setExpanded((v) => !v)}
              className="mt-1 text-brand-muted underline hover:text-brand-fg"
            >
              {expanded ? "Show less" : "Show more"}
            </button>
          )}
        </div>
      ) : (
        <p className="mt-2 text-xs text-brand-muted">No release notes.</p>
      )}
      {isDownloading && (
        <div className="mt-2">
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
    </li>
  );
}

