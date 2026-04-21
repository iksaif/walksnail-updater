import { ipc } from "@/lib/ipc";
import { useStore } from "@/state/store";

export default function About() {
  const setView = useStore((s) => s.setView);
  return (
    <section className="max-w-xl space-y-4">
      <header>
        <button
          onClick={() => setView({ kind: "home" })}
          className="text-xs text-brand-muted hover:text-brand-fg"
        >
          ← Back
        </button>
        <h2 className="text-2xl font-semibold">About</h2>
      </header>
      <div className="space-y-3 rounded border border-brand-muted/30 p-4 text-sm">
        <p>
          <strong>Walksnail Updater</strong> is an unofficial, community-built
          tool for flashing Walksnail Avatar HD firmware.
        </p>
        <p>
          <strong>
            Not affiliated with, endorsed by, or sponsored by Walksnail,
            CADDXFPV, or any related entity.
          </strong>{" "}
          All trademarks remain the property of their respective owners.
        </p>
        <p>
          Huge thanks to{" "}
          <button
            onClick={() => ipc.openUrl("https://d3vl.com")}
            className="underline hover:text-brand-fg"
          >
            D3VL
          </button>{" "}
          for running{" "}
          <button
            onClick={() => ipc.openUrl("https://walksnail.app")}
            className="underline hover:text-brand-fg"
          >
            walksnail.app
          </button>{" "}
          and publishing the firmware mirror — the app's index comes from
          there. Official downloads:{" "}
          <button
            onClick={() =>
              ipc.openUrl("https://www.caddxfpv.com/pages/download-center")
            }
            className="underline hover:text-brand-fg"
          >
            CADDXFPV Download Center
          </button>
          . Community wiki:{" "}
          <button
            onClick={() => ipc.openUrl("https://walksnail.wiki")}
            className="underline hover:text-brand-fg"
          >
            walksnail.wiki
          </button>
          .
        </p>
        <p className="text-xs text-brand-muted">
          Licensed under MIT. No warranty: use at your own risk.
        </p>
      </div>
    </section>
  );
}
