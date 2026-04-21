export default function LoadingOverlay({ message }: { message: string }) {
  return (
    <div
      className="fixed inset-0 z-40 flex items-center justify-center bg-brand-bg/70 backdrop-blur-sm"
      role="status"
      aria-live="polite"
    >
      <div className="flex flex-col items-center gap-3 rounded-lg border border-brand-muted/30 bg-black/40 px-6 py-5 shadow-xl">
        <span className="inline-block h-8 w-8 animate-spin rounded-full border-2 border-brand-muted/30 border-t-brand-accent" />
        <span className="text-sm text-brand-dim">{message}</span>
      </div>
    </div>
  );
}
