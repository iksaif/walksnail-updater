import { useState } from "react";
import { useStore } from "@/state/store";

export default function Onboarding() {
  const finish = useStore((s) => s.finishOnboarding);
  const [slide, setSlide] = useState(0);
  const [ack, setAck] = useState(false);
  const slides = [
    {
      title: "Update your Walksnail hardware safely",
      body: (
        <ul className="list-disc pl-5 text-sm space-y-1">
          <li>Auto-detects your hardware when you plug in its SD card.</li>
          <li>Downloads the right firmware, verifies its SHA1, and copies it for you.</li>
          <li>Browse every past release and read the changelog.</li>
        </ul>
      ),
    },
    {
      title: "Safety first",
      body: (
        <div className="text-sm space-y-2">
          <p>
            <strong>Goggles X</strong> manufactured since Jan 2025 must stay at
            version <span className="font-mono">38.44.13</span> or higher.
            Flashing anything lower is a permanent brick — the app refuses to
            stage such firmware.
          </p>
          <p>
            <strong>Avatar GT</strong> units require at least{" "}
            <span className="font-mono">39.44.2</span>. We surface a warning
            before any generic downgrade.
          </p>
        </div>
      ),
    },
    {
      title: "Not affiliated with Walksnail",
      body: (
        <div className="text-sm space-y-2">
          <p>
            This is an independent community tool. <strong>Not affiliated with,
            endorsed by, or sponsored by Walksnail, CADDXFPV, or any related
            entity.</strong> All trademarks remain the property of their
            respective owners.
          </p>
          <p>
            Firmware is fetched from{" "}
            <span className="font-mono">walksnail.app</span> (run by{" "}
            <span className="font-medium">D3VL</span>, thanks!) and the
            CADDXFPV download page. Files are not modified. Flash at your own
            risk; the authors accept no liability for damaged hardware.
          </p>
          <label className="mt-4 flex items-center gap-2">
            <input
              type="checkbox"
              checked={ack}
              onChange={(e) => setAck(e.target.checked)}
            />
            <span>I understand and accept the risk.</span>
          </label>
        </div>
      ),
    },
  ];

  const current = slides[slide];
  const isLast = slide === slides.length - 1;

  return (
    <div className="flex h-full items-center justify-center p-6">
      <div className="w-full max-w-lg space-y-6 rounded-lg border border-brand-muted/30 bg-white/5 p-6">
        <div className="flex gap-1">
          {slides.map((_, i) => (
            <span
              key={i}
              className={`h-1 flex-1 rounded ${
                i <= slide ? "bg-brand-accent" : "bg-brand-muted/20"
              }`}
            />
          ))}
        </div>
        <h2 className="text-xl font-semibold">{current.title}</h2>
        <div>{current.body}</div>
        <div className="flex justify-between">
          <button
            onClick={() => setSlide(Math.max(0, slide - 1))}
            disabled={slide === 0}
            className="text-sm text-brand-muted hover:text-brand-fg"
          >
            Back
          </button>
          {isLast ? (
            <button
              onClick={() => ack && finish()}
              disabled={!ack}
              className="rounded bg-brand-accent px-4 py-2 text-sm font-medium text-brand-bg hover:brightness-110"
            >
              Continue
            </button>
          ) : (
            <button
              onClick={() => setSlide(slide + 1)}
              className="rounded bg-brand-accent px-4 py-2 text-sm font-medium text-brand-bg hover:brightness-110"
            >
              Next
            </button>
          )}
        </div>
      </div>
    </div>
  );
}
