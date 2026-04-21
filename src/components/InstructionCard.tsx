import { Hardware, HARDWARE_LABELS, HardwareInstructions, ipc } from "@/lib/ipc";

export default function InstructionCard({
  hardware,
  instructions,
}: {
  hardware: Hardware;
  instructions: HardwareInstructions;
}) {
  return (
    <div className="rounded border border-brand-muted/30 bg-white/5 p-4">
      <div className="flex items-center justify-between">
        <h3 className="font-medium">
          Next steps · {HARDWARE_LABELS[hardware]}
        </h3>
        <button
          onClick={() => window.print()}
          className="text-xs text-brand-muted underline hover:text-brand-fg"
        >
          Print
        </button>
      </div>
      <ol className="mt-3 list-decimal space-y-2 pl-5 text-sm">
        {instructions.steps.map((s, i) => (
          <li key={i}>{s}</li>
        ))}
      </ol>
      <p className="mt-4 text-xs text-brand-muted">
        Summarised from the public procedure.{" "}
        <button
          onClick={() => ipc.openUrl(instructions.source_url)}
          className="underline hover:text-brand-fg"
        >
          Open the full manual
        </button>
        .
      </p>
    </div>
  );
}
