import { open as openDialog } from "@tauri-apps/plugin-dialog";
import { HARDWARE_LABELS, versionString } from "@/lib/ipc";
import { useStore } from "@/state/store";
import HardwareIcon from "@/components/HardwareIcon";
import { FolderIcon, RefreshIcon, WarningIcon } from "@/components/Icons";

export default function SdCardPanel() {
  const sdMounts = useStore((s) => s.sdMounts);
  const activeSd = useStore((s) => s.activeSd);
  const setActiveSd = useStore((s) => s.setActiveSd);
  const addManualMount = useStore((s) => s.addManualMount);
  const scan = useStore((s) => s.sdScan);
  const rescan = useStore((s) => s.rescanSd);

  async function pickSdManually() {
    const picked = await openDialog({
      directory: true,
      multiple: false,
      title: "Pick your SD card's mount point",
    });
    if (typeof picked === "string") {
      addManualMount(picked);
    }
  }

  const pickBtn = (
    <button
      onClick={pickSdManually}
      className="inline-flex items-center gap-1 text-xs text-brand-muted underline hover:text-brand-fg"
      title="Manually point at an SD card that the auto-detector missed"
    >
      <FolderIcon className="h-3.5 w-3.5 text-brand-accent" /> Pick SD card…
    </button>
  );

  if (sdMounts.length === 0) {
    return (
      <div className="flex flex-wrap items-center gap-3 rounded border border-dashed border-brand-muted/30 p-3 text-sm text-brand-dim">
        <span>
          No removable drive detected. Plug in an SD card to auto-detect your
          Walksnail hardware — or download firmware without one.
        </span>
        {pickBtn}
      </div>
    );
  }

  const picker = sdMounts.length > 1 && (
    <label className="ml-auto flex items-center gap-2 text-xs text-brand-muted">
      <span>Drive</span>
      <select
        value={activeSd ?? ""}
        onChange={(e) => setActiveSd(e.target.value)}
        className="rounded border border-brand-muted/30 bg-black/30 px-2 py-1 text-brand-fg"
      >
        {sdMounts.map((p) => (
          <option key={p} value={p}>
            {p}
          </option>
        ))}
      </select>
    </label>
  );

  if (!scan) {
    return (
      <div className="flex flex-wrap items-center gap-3 rounded border border-brand-muted/30 bg-white/5 p-3 text-sm">
        <span>Scanning {activeSd ?? sdMounts[0]}…</span>
        {pickBtn}
        {picker}
      </div>
    );
  }

  const c = scan.contents;
  if (!c.is_walksnail) {
    return (
      <div className="flex flex-wrap items-center gap-3 rounded border border-brand-muted/30 bg-white/5 p-3 text-sm">
        <div className="min-w-0">
          <div className="font-medium">SD mounted, no Walksnail firmware detected</div>
          <p className="mt-1 text-xs text-brand-dim">{c.root}</p>
        </div>
        <button
          onClick={() => rescan(c.root)}
          className="ml-auto inline-flex items-center gap-1 text-xs text-brand-muted underline hover:text-brand-fg"
        >
          <RefreshIcon className="h-3.5 w-3.5" /> Rescan
        </button>
        {pickBtn}
        {picker}
      </div>
    );
  }

  const running = c.running_version && versionString(c.running_version);
  const staged = c.staged_version && versionString(c.staged_version);
  const latest =
    scan.latest_stable && versionString(scan.latest_stable.version);
  const hwLabel = c.variant ? HARDWARE_LABELS[c.variant] : "Walksnail SD";

  return (
    <div className="rounded border border-brand-accent bg-brand-accent/5 p-3 text-sm">
      <div className="flex flex-wrap items-center gap-x-4 gap-y-1">
        {c.variant && (
          <HardwareIcon hardware={c.variant} className="h-10 w-10 text-brand-accent" />
        )}
        <span className="font-medium">Detected: {hwLabel}</span>
        {running && (
          <span>
            Running <span className="font-mono">{running}</span>
          </span>
        )}
        {staged && (
          <span>
            Staged <span className="font-mono">{staged}</span>
          </span>
        )}
        {latest && (
          <span>
            Latest stable <span className="font-mono">{latest}</span>
          </span>
        )}
        <span className="text-xs text-brand-muted">{c.root}</span>
        <button
          onClick={() => rescan(c.root)}
          className="inline-flex items-center gap-1 text-xs text-brand-muted underline hover:text-brand-fg"
          title="Rescan the SD card"
        >
          <RefreshIcon className="h-3.5 w-3.5" /> Rescan
        </button>
        {pickBtn}
        {picker}
      </div>
      {scan.verdict && scan.verdict.kind !== "ok" && (
        <p className="mt-2 flex items-start gap-2 text-xs">
          <WarningIcon
            className={`mt-0.5 h-4 w-4 shrink-0 ${
              scan.verdict.kind === "block" ? "text-brand-danger" : "text-brand-warn"
            }`}
          />
          <span>
            <strong
              className={
                scan.verdict.kind === "block" ? "text-brand-danger" : "text-brand-warn"
              }
            >
              {scan.verdict.kind === "block" ? "Blocked: " : "Warning: "}
            </strong>
            {scan.verdict.reason}
          </span>
        </p>
      )}
    </div>
  );
}
