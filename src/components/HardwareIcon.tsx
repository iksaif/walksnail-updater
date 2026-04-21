// Tiny hardware silhouettes so users can spot their device visually.
// Deliberately geometric and monochrome — these are *not* Walksnail product
// photos, they're icons evoking the category (goggles, VTX, VRX, relay).

import { SVGProps } from "react";
import { Hardware } from "@/lib/ipc";

function Frame({ children, ...rest }: SVGProps<SVGSVGElement> & { children: React.ReactNode }) {
  return (
    <svg
      viewBox="0 0 48 48"
      fill="none"
      stroke="currentColor"
      strokeWidth={1.75}
      strokeLinecap="round"
      strokeLinejoin="round"
      aria-hidden="true"
      {...rest}
    >
      {children}
    </svg>
  );
}

const Goggles = (p: SVGProps<SVGSVGElement>) => (
  <Frame {...p}>
    {/* Two lens circles, bridge, head strap */}
    <rect x="4" y="14" width="40" height="18" rx="5" />
    <circle cx="15" cy="23" r="5" />
    <circle cx="33" cy="23" r="5" />
    <path d="M20 23h8" />
    <path d="M4 20 1 18M44 20l3-2" />
  </Frame>
);

const GogglesX = (p: SVGProps<SVGSVGElement>) => (
  <Frame {...p}>
    <rect x="3" y="13" width="42" height="20" rx="3" />
    <rect x="8" y="18" width="12" height="10" rx="1" />
    <rect x="28" y="18" width="12" height="10" rx="1" />
    <path d="M3 17 0 15M45 17l3-2" />
  </Frame>
);

const GogglesL = (p: SVGProps<SVGSVGElement>) => (
  <Frame {...p}>
    <rect x="5" y="15" width="38" height="17" rx="8" />
    <circle cx="16" cy="23" r="4" />
    <circle cx="32" cy="23" r="4" />
    <path d="M5 20 2 18M43 20l3-2" />
  </Frame>
);

const SkyVtx = (p: SVGProps<SVGSVGElement>) => (
  <Frame {...p}>
    {/* PCB rectangle with antenna coax tail and lens */}
    <rect x="8" y="14" width="22" height="22" rx="2" />
    <circle cx="19" cy="25" r="4" />
    <path d="M30 18h8l4-4" />
    <path d="M30 32h6" />
  </Frame>
);

const MiniSky = (p: SVGProps<SVGSVGElement>) => (
  <Frame {...p}>
    <rect x="12" y="16" width="18" height="18" rx="2" />
    <circle cx="21" cy="25" r="3" />
    <path d="M30 20h8l3-3" />
  </Frame>
);

const Moonlight = (p: SVGProps<SVGSVGElement>) => (
  <Frame {...p}>
    {/* Box + separate camera + wire link */}
    <rect x="5" y="16" width="18" height="16" rx="2" />
    <circle cx="14" cy="24" r="3.5" />
    <rect x="30" y="20" width="12" height="8" rx="2" />
    <path d="M23 24h7" />
    <path d="M42 18v-4h-4" />
  </Frame>
);

const Vrx = (p: SVGProps<SVGSVGElement>) => (
  <Frame {...p}>
    <rect x="6" y="18" width="26" height="14" rx="2" />
    <path d="M32 22h6l4-4" />
    <path d="M32 28h6l4 4" />
    <circle cx="12" cy="25" r="1" />
    <circle cx="16" cy="25" r="1" />
    <circle cx="20" cy="25" r="1" />
  </Frame>
);

const Relay = (p: SVGProps<SVGSVGElement>) => (
  <Frame {...p}>
    {/* Tower-with-signal motif */}
    <path d="M24 8v28" />
    <path d="M18 40h12" />
    <path d="M16 14a10 10 0 0 0 0 14" />
    <path d="M32 14a10 10 0 0 1 0 14" />
    <path d="M20 18a4 4 0 0 0 0 6" />
    <path d="M28 18a4 4 0 0 1 0 6" />
  </Frame>
);

export default function HardwareIcon({
  hardware,
  ...rest
}: { hardware: Hardware } & SVGProps<SVGSVGElement>) {
  switch (hardware) {
    case "AvatarSky":
      return <SkyVtx {...rest} />;
    case "AvatarMiniSky":
      return <MiniSky {...rest} />;
    case "MoonlightSky":
      return <Moonlight {...rest} />;
    case "AvatarGnd":
      return <Goggles {...rest} />;
    case "GogglesX":
      return <GogglesX {...rest} />;
    case "GogglesL":
      return <GogglesL {...rest} />;
    case "VrxSE":
      return <Vrx {...rest} />;
    case "ReconHd":
      return <Goggles {...rest} />;
    case "AvatarRelay":
      return <Relay {...rest} />;
  }
}
