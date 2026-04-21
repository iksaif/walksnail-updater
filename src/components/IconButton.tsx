// Icon-only button with an accessible label + tooltip.

import { ReactNode } from "react";

interface Props {
  label: string;
  onClick: () => void;
  disabled?: boolean;
  variant?: "default" | "primary" | "success" | "info" | "ghost";
  title?: string;
  children: ReactNode;
}

export default function IconButton({
  label,
  onClick,
  disabled = false,
  variant = "default",
  title,
  children,
}: Props) {
  const base =
    "inline-flex items-center justify-center h-9 w-9 rounded-md transition disabled:opacity-40 disabled:cursor-not-allowed";
  const kind =
    variant === "primary"
      ? "bg-brand-accent text-brand-bg hover:brightness-110"
      : variant === "success"
        ? "bg-brand-ok text-brand-bg hover:brightness-110"
        : variant === "info"
          ? "bg-brand-info text-brand-bg hover:brightness-110"
          : variant === "ghost"
            ? "text-brand-muted hover:bg-white/10 hover:text-brand-fg"
            : "border border-brand-muted/40 text-brand-fg hover:bg-white/5";
  return (
    <button
      type="button"
      onClick={onClick}
      disabled={disabled}
      aria-label={label}
      title={title ?? label}
      className={`${base} ${kind}`}
    >
      {children}
    </button>
  );
}
