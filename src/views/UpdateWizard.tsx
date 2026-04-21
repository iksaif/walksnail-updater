import { useEffect, useMemo, useState } from "react";
import {
  Hardware,
  SafetyVerdict,
  StageProgress,
  Version,
  cmpVersion,
  ipc,
  onStageProgress,
  versionString,
  HARDWARE_LABELS,
} from "@/lib/ipc";
import { useStore } from "@/state/store";
import InstructionCard from "@/components/InstructionCard";

type Step = "confirm" | "stage" | "done" | "error";

export default function UpdateWizard({
  hardware,
  version,
}: {
  hardware: Hardware;
  version: Version;
}) {
  const index = useStore((s) => s.index);
  const sdScan = useStore((s) => s.sdScan);
  const setView = useStore((s) => s.setView);
  const instructions = useStore((s) => s.instructions[hardware]);

  const release = useMemo(
    () =>
      index?.releases.find(
        (r) =>
          cmpVersion(r.version, version) === 0 &&
          r.downloads.some((d) => d.hardware === hardware),
      ) ?? null,
    [index, hardware, version],
  );

  const current = sdScan?.contents.running_version ?? sdScan?.contents.staged_version ?? null;

  const verdict: SafetyVerdict | null = useMemo(() => {
    if (!release) return null;
    // Mirror the Rust safety_check client-side for immediate feedback; the
    // Rust call on stage will still double-check.
    const floor = GOGGLES_X_FLOOR;
    if (hardware === "GogglesX" && cmpVersion(version, floor) < 0) {
      return {
        kind: "block",
        reason: `Goggles X flashing below ${versionString(floor)} is a known brick. Refused.`,
      };
    }
    if (current && cmpVersion(version, current) < 0) {
      return {
        kind: "warn",
        reason: `Downgrade from ${versionString(current)} to ${versionString(version)}.`,
      };
    }
    return { kind: "ok" };
  }, [hardware, version, current, release]);

  const [step, setStep] = useState<Step>("confirm");
  const [confirmText, setConfirmText] = useState("");
  const [progress, setProgress] = useState<StageProgress | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [moonlightDvrOnly, setMoonlightDvrOnly] = useState(false);
  const sdPath = sdScan?.contents.root ?? null;

  useEffect(() => {
    let unlisten: (() => void) | undefined;
    onStageProgress((p) => {
      setProgress(p);
      if (p.kind === "done") setStep("done");
    }).then((fn) => {
      unlisten = fn;
    });
    return () => {
      unlisten?.();
    };
  }, []);

  if (!release) {
    return <p className="text-sm text-brand-danger">Release {versionString(version)} not found for {HARDWARE_LABELS[hardware]}.</p>;
  }

  async function start() {
    if (!sdPath) {
      setError("No SD card mounted.");
      setStep("error");
      return;
    }
    setStep("stage");
    try {
      await ipc.stageFirmware(hardware, version, sdPath, moonlightDvrOnly);
    } catch (e) {
      setError(String(e));
      setStep("error");
    }
  }

  const downgradeOk =
    verdict?.kind !== "warn" || confirmText.trim() === "I understand — downgrade";

  return (
    <section className="mx-auto max-w-2xl space-y-4">
      <header>
        <button
          onClick={() => setView({ kind: "home" })}
          className="text-xs text-brand-muted hover:text-brand-fg"
        >
          ← Cancel
        </button>
        <h2 className="text-2xl font-semibold">
          Stage {versionString(version)} → {HARDWARE_LABELS[hardware]}
        </h2>
      </header>

      {step === "confirm" && (
        <div className="space-y-3">
          <div className="rounded border border-brand-muted/30 bg-white/5 p-3 text-sm">
            <div>Target SD: <span className="font-mono">{sdPath ?? "(none)"}</span></div>
            {current && (
              <div>
                Current version on SD:{" "}
                <span className="font-mono">{versionString(current)}</span>
              </div>
            )}
            <details className="mt-2 text-xs text-brand-muted">
              <summary className="cursor-pointer">Release notes</summary>
              <pre className="whitespace-pre-wrap">{release.notes || "—"}</pre>
            </details>
          </div>

          {verdict?.kind === "block" && (
            <div className="rounded border border-brand-danger/40 bg-brand-danger/10 p-3 text-sm">
              <strong>Refused:</strong> {verdict.reason}
            </div>
          )}
          {verdict?.kind === "warn" && (
            <div className="rounded border border-brand-warn/40 bg-brand-warn/10 p-3 text-sm">
              <strong>Warning:</strong> {verdict.reason}
              <p className="mt-2">
                Type <code>I understand — downgrade</code> to continue:
              </p>
              <input
                className="mt-1 w-full rounded bg-black/30 px-2 py-1 font-mono"
                value={confirmText}
                onChange={(e) => setConfirmText(e.target.value)}
                placeholder="I understand — downgrade"
              />
            </div>
          )}

          {hardware === "MoonlightSky" && (
            <label className="flex items-center gap-2 text-sm">
              <input
                type="checkbox"
                checked={moonlightDvrOnly}
                onChange={(e) => setMoonlightDvrOnly(e.target.checked)}
              />
              Moonlight DVR-only upgrade (writes <code>independ_upgrade.txt</code>)
            </label>
          )}

          <button
            onClick={start}
            disabled={verdict?.kind === "block" || !sdPath || !downgradeOk}
            className="rounded bg-brand-accent px-4 py-2 font-medium text-brand-bg hover:brightness-110"
          >
            Stage firmware
          </button>
        </div>
      )}

      {step === "stage" && (
        <div className="rounded border border-brand-muted/30 bg-white/5 p-3 text-sm">
          <Progress p={progress} />
        </div>
      )}

      {step === "done" && (
        <div className="space-y-4">
          <div className="rounded border border-brand-ok/40 bg-brand-ok/10 p-3 text-sm">
            Firmware staged. Safely eject the SD and follow the steps below.
          </div>
          {instructions ? (
            <InstructionCard hardware={hardware} instructions={instructions} />
          ) : (
            <p className="text-sm text-brand-muted">
              No bundled instructions for this device — consult the upstream
              manual before continuing.
            </p>
          )}
        </div>
      )}

      {step === "error" && (
        <div className="rounded border border-brand-danger/40 bg-brand-danger/10 p-3 text-sm">
          {error}
        </div>
      )}
    </section>
  );
}

function Progress({ p }: { p: StageProgress | null }) {
  if (!p) return <p>Starting…</p>;
  if (p.kind === "downloading") {
    const pct = p.total ? Math.round((p.received / p.total) * 100) : null;
    return <p>Downloading{pct !== null ? ` ${pct}%` : `… ${fmt(p.received)}`}</p>;
  }
  if (p.kind === "verifying") return <p>Verifying SHA1…</p>;
  if (p.kind === "copying") return <p>Copying to SD…</p>;
  return <p>Done.</p>;
}

function fmt(bytes: number) {
  const units = ["B", "KiB", "MiB", "GiB"];
  let i = 0;
  let n = bytes;
  while (n >= 1024 && i < units.length - 1) {
    n /= 1024;
    i++;
  }
  return `${n.toFixed(1)} ${units[i]}`;
}

const GOGGLES_X_FLOOR = { major: 38, minor: 44, patch: 13 };
