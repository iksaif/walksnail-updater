import { useEffect, useRef, useState } from "react";

interface Props {
  received: number;
  total: number | null;
  /// When true, show an indeterminate shimmer without any bytes/% text.
  verifying?: boolean;
}

const BYTE_UNITS = ["B", "KiB", "MiB", "GiB"] as const;

function fmtBytes(n: number): string {
  let i = 0;
  let v = n;
  while (v >= 1024 && i < BYTE_UNITS.length - 1) {
    v /= 1024;
    i++;
  }
  return `${v.toFixed(i === 0 ? 0 : 1)} ${BYTE_UNITS[i]}`;
}

function fmtRate(bytesPerSec: number): string {
  if (!isFinite(bytesPerSec) || bytesPerSec <= 0) return "—";
  return `${fmtBytes(bytesPerSec)}/s`;
}

/**
 * Progress bar with three modes:
 *
 * 1. **Verifying** — indeterminate striped animation, no text.
 * 2. **Known total** — percentage bar, smoothly tweened toward the target
 *    width with requestAnimationFrame so many small backend updates render
 *    as one continuous motion.
 * 3. **Unknown total** — indeterminate shimmer with a live rate (bytes/s)
 *    and total downloaded so far. No fake percentage.
 */
export default function ProgressBar({ received, total, verifying = false }: Props) {
  const hasTotal = total !== null && total > 0;
  const targetPct = hasTotal ? Math.min(100, (received / total) * 100) : 0;

  const [displayPct, setDisplayPct] = useState(targetPct);
  const displayRef = useRef(displayPct);
  const targetRef = useRef(targetPct);
  const rafRef = useRef<number | null>(null);

  useEffect(() => {
    targetRef.current = targetPct;
    if (!hasTotal || verifying) {
      // No smoothing needed for indeterminate / verifying modes.
      return;
    }
    const tick = () => {
      const diff = targetRef.current - displayRef.current;
      if (Math.abs(diff) < 0.1) {
        displayRef.current = targetRef.current;
        setDisplayPct(targetRef.current);
        rafRef.current = null;
        return;
      }
      // Ease-out: close 15 % of the remaining gap per frame.
      const step = diff * 0.15;
      displayRef.current += step;
      setDisplayPct(displayRef.current);
      rafRef.current = requestAnimationFrame(tick);
    };
    if (rafRef.current === null) {
      rafRef.current = requestAnimationFrame(tick);
    }
    return () => {
      if (rafRef.current !== null) {
        cancelAnimationFrame(rafRef.current);
        rafRef.current = null;
      }
    };
  }, [targetPct, hasTotal, verifying]);

  // Rolling rate estimation: compare `received` against a few samples back.
  const samplesRef = useRef<{ t: number; bytes: number }[]>([]);
  const [rate, setRate] = useState(0);
  useEffect(() => {
    const now = performance.now();
    const samples = samplesRef.current;
    samples.push({ t: now, bytes: received });
    while (samples.length > 0 && now - samples[0].t > 1500) {
      samples.shift();
    }
    if (samples.length >= 2) {
      const first = samples[0];
      const last = samples[samples.length - 1];
      const dt = (last.t - first.t) / 1000;
      setRate(dt > 0 ? (last.bytes - first.bytes) / dt : 0);
    }
  }, [received]);

  if (verifying) {
    return (
      <div className="flex items-center gap-2 text-xs text-brand-muted">
        <IndeterminateBar />
        <span>Verifying…</span>
      </div>
    );
  }

  if (!hasTotal) {
    return (
      <div className="flex items-center gap-2 text-xs text-brand-muted">
        <IndeterminateBar />
        <span className="tabular-nums whitespace-nowrap">
          {fmtBytes(received)} · {fmtRate(rate)}
        </span>
      </div>
    );
  }

  return (
    <div className="flex items-center gap-2 text-xs text-brand-muted">
      <div className="h-1.5 flex-1 overflow-hidden rounded bg-white/10">
        <div
          className="h-full rounded bg-brand-ok"
          style={{ width: `${displayPct}%` }}
        />
      </div>
      <span className="tabular-nums whitespace-nowrap">
        {Math.floor(displayPct)}% · {fmtRate(rate)}
      </span>
    </div>
  );
}

function IndeterminateBar() {
  return (
    <div className="relative h-1.5 flex-1 overflow-hidden rounded bg-white/10">
      <div className="absolute inset-y-0 left-0 w-1/3 animate-progress-indeterminate rounded bg-brand-ok" />
    </div>
  );
}
