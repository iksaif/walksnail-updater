import { useEffect, useState } from "react";
import { open as openDialog } from "@tauri-apps/plugin-dialog";
import { ipc } from "@/lib/ipc";
import { useStore } from "@/state/store";
import { FolderIcon, RefreshIcon } from "@/components/Icons";

export default function Settings() {
  const reloadIndex = useStore((s) => s.reloadIndex);
  const setView = useStore((s) => s.setView);
  const downloadDir = useStore((s) => s.downloadDir);
  const reloadDownloadDir = useStore((s) => s.reloadDownloadDir);
  const [paths, setPaths] = useState<{ data_dir: string; cache_dir: string } | null>(null);
  const [pref, setPref] = useState<string | null>(null);
  const [refreshing, setRefreshing] = useState(false);

  useEffect(() => {
    Promise.all([ipc.getAppPaths(), ipc.getDownloadDirPref()]).then(([p, pr]) => {
      setPaths(p);
      setPref(pr);
    });
  }, [downloadDir]);

  async function pickDir() {
    const picked = await openDialog({
      directory: true,
      multiple: false,
      title: "Choose download directory",
    });
    if (typeof picked === "string") {
      await ipc.setDownloadDir(picked);
      await reloadDownloadDir();
    }
  }

  async function resetDir() {
    await ipc.setDownloadDir(null);
    await reloadDownloadDir();
  }

  return (
    <section className="max-w-xl space-y-4">
      <header>
        <button
          onClick={() => setView({ kind: "home" })}
          className="text-xs text-brand-muted hover:text-brand-fg"
        >
          ← Back
        </button>
        <h2 className="text-2xl font-semibold">Settings</h2>
      </header>

      <div className="space-y-3 rounded border border-brand-muted/30 p-4 text-sm">
        <div className="flex items-center justify-between gap-3">
          <div>
            <div className="font-medium">Firmware index</div>
            <div className="text-xs text-brand-muted">
              Primary: walksnail.app. Fallbacks: D3VL mirror JSON, CADDXFPV
              download page.
            </div>
          </div>
          <button
            onClick={async () => {
              setRefreshing(true);
              await reloadIndex();
              setRefreshing(false);
            }}
            disabled={refreshing}
            className="inline-flex items-center gap-2 rounded border border-brand-muted/40 px-3 py-1 text-xs hover:bg-white/5"
          >
            <RefreshIcon className="h-3.5 w-3.5" />
            {refreshing ? "Refreshing…" : "Refresh now"}
          </button>
        </div>
      </div>

      <div className="space-y-3 rounded border border-brand-muted/30 p-4 text-sm">
        <div className="flex items-start justify-between gap-3">
          <div className="min-w-0">
            <div className="font-medium">Download directory</div>
            <div className="truncate font-mono text-xs text-brand-dim">
              {downloadDir ?? "…"}
            </div>
            {pref && (
              <div className="mt-0.5 text-xs text-brand-muted">
                (custom — default is{" "}
                <span className="font-mono text-brand-dim">{paths?.cache_dir}</span>)
              </div>
            )}
          </div>
          <div className="flex gap-2">
            <button
              onClick={pickDir}
              className="inline-flex items-center gap-2 rounded border border-brand-muted/40 px-3 py-1 text-xs hover:bg-white/5"
            >
              <FolderIcon className="h-4 w-4 text-brand-accent" /> Choose…
            </button>
            {pref && (
              <button
                onClick={resetDir}
                className="rounded px-3 py-1 text-xs text-brand-muted hover:text-brand-fg"
              >
                Reset to default
              </button>
            )}
          </div>
        </div>
      </div>

      <div className="space-y-1 rounded border border-brand-muted/30 p-4 text-sm">
        <div className="font-medium">Storage</div>
        <div className="text-xs text-brand-dim">
          Data dir: <span className="font-mono">{paths?.data_dir ?? "…"}</span>
        </div>
      </div>

      <button
        onClick={() => setView({ kind: "about" })}
        className="text-sm text-brand-muted underline hover:text-brand-fg"
      >
        About · disclaimers · credits
      </button>
    </section>
  );
}
