import { useStore } from "@/state/store";
import { HomeIcon, InfoIcon, SettingsIcon } from "@/components/Icons";

export default function DisclaimerBanner() {
  const view = useStore((s) => s.view);
  const setView = useStore((s) => s.setView);

  const active = (kind: string) =>
    view.kind === kind
      ? "text-brand-fg"
      : "text-brand-muted hover:text-brand-fg";

  return (
    <header className="flex items-center gap-1 border-b border-brand-muted/20 bg-black/40 px-4 py-1.5 text-xs">
      <NavButton
        label="Home"
        onClick={() => setView({ kind: "home" })}
        className={active("home")}
      >
        <HomeIcon className="h-4 w-4" /> Home
      </NavButton>
      <NavButton
        label="Settings"
        onClick={() => setView({ kind: "settings" })}
        className={active("settings")}
      >
        <SettingsIcon className="h-4 w-4" /> Settings
      </NavButton>
      <NavButton
        label="About"
        onClick={() => setView({ kind: "about" })}
        className={active("about")}
      >
        <InfoIcon className="h-4 w-4" /> About
      </NavButton>
    </header>
  );
}

function NavButton({
  label,
  onClick,
  className,
  children,
}: {
  label: string;
  onClick: () => void;
  className: string;
  children: React.ReactNode;
}) {
  return (
    <button
      onClick={onClick}
      aria-label={label}
      className={`inline-flex items-center gap-1.5 rounded px-2 py-1 transition ${className}`}
    >
      {children}
    </button>
  );
}
