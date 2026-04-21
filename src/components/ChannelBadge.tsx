import { Channel } from "@/lib/ipc";

export default function ChannelBadge({ channel }: { channel: Channel }) {
  const color =
    channel === "stable"
      ? "bg-brand-ok/20 text-brand-ok"
      : channel === "beta"
        ? "bg-brand-warn/20 text-brand-warn"
        : "bg-white/10 text-brand-muted";
  return (
    <span
      className={`rounded px-1.5 py-0.5 text-[10px] uppercase tracking-wide ${color}`}
    >
      {channel}
    </span>
  );
}
