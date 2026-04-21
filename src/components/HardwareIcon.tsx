// Device silhouettes — hand-authored, inspired by the product shapes on
// walksnail.app (D3VL). They aren't exact recreations; the goal is "I can
// tell at a glance which device this is" without claiming to be the
// manufacturer's artwork.
//
// All glyphs share a 48×48 viewBox and use `currentColor`, so Tailwind
// `text-*` classes recolour them.

import { SVGProps } from "react";
import { Hardware } from "@/lib/ipc";

type Props = { hardware: Hardware } & SVGProps<SVGSVGElement>;

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

/** Avatar HD Goggles (V1): two round lenses + chunky frame, no head-strap. */
const AvatarGnd = (p: SVGProps<SVGSVGElement>) => (
  <Frame {...p}>
    <path d="M5 18h38a3 3 0 0 1 3 3v10a3 3 0 0 1-3 3H5a3 3 0 0 1-3-3V21a3 3 0 0 1 3-3z" />
    <circle cx="15" cy="26" r="5" />
    <circle cx="33" cy="26" r="5" />
    <path d="M21 26h6" />
  </Frame>
);

/** Goggles X: rectangular lens windows, bevelled corners, prominent bridge. */
const GogglesX = (p: SVGProps<SVGSVGElement>) => (
  <Frame {...p}>
    <path d="M3 16a3 3 0 0 1 3-3h36a3 3 0 0 1 3 3v16a3 3 0 0 1-3 3H6a3 3 0 0 1-3-3z" />
    <rect x="8" y="19" width="13" height="10" rx="1.5" />
    <rect x="27" y="19" width="13" height="10" rx="1.5" />
  </Frame>
);

/** Goggles L (Lite): two round lenses in a soft rounded frame, thinner body. */
const GogglesL = (p: SVGProps<SVGSVGElement>) => (
  <Frame {...p}>
    <path d="M4 20c0-4 3-7 7-7h26c4 0 7 3 7 7v8c0 4-3 7-7 7H11c-4 0-7-3-7-7z" />
    <circle cx="16" cy="24" r="4.5" />
    <circle cx="32" cy="24" r="4.5" />
  </Frame>
);

/** Recon HD: goggles-style body with a mic/status dot in the nose bridge. */
const ReconHd = (p: SVGProps<SVGSVGElement>) => (
  <Frame {...p}>
    <path d="M4 18h40a2 2 0 0 1 2 2v10a3 3 0 0 1-3 3H5a3 3 0 0 1-3-3V20a2 2 0 0 1 2-2z" />
    <circle cx="15" cy="26" r="4.5" />
    <circle cx="33" cy="26" r="4.5" />
    <circle cx="24" cy="26" r="1" fill="currentColor" />
  </Frame>
);

/** Avatar HD VTX: square PCB with a camera lens and two antenna pigtails. */
const AvatarSky = (p: SVGProps<SVGSVGElement>) => (
  <Frame {...p}>
    <rect x="10" y="14" width="22" height="22" rx="2" />
    <circle cx="21" cy="25" r="4.5" />
    <circle cx="21" cy="25" r="1.5" fill="currentColor" />
    <path d="M32 18h6l4-4" />
    <path d="M32 32h6l4 4" />
  </Frame>
);

/** Avatar Mini 1S VTX: smaller PCB, single antenna pigtail. */
const AvatarMiniSky = (p: SVGProps<SVGSVGElement>) => (
  <Frame {...p}>
    <rect x="14" y="17" width="16" height="16" rx="2" />
    <circle cx="22" cy="25" r="3.5" />
    <circle cx="22" cy="25" r="1" fill="currentColor" />
    <path d="M30 22h8l4-3" />
  </Frame>
);

/** Moonlight 4K VTX: round camera-module shape. */
const MoonlightSky = (p: SVGProps<SVGSVGElement>) => (
  <Frame {...p}>
    <circle cx="24" cy="24" r="17" />
    <circle cx="24" cy="24" r="10" />
    <circle cx="24" cy="24" r="4.5" />
    <circle cx="24" cy="24" r="1.5" fill="currentColor" />
    {/* antenna coax exiting the module */}
    <path d="M41 24h4l3-3" />
  </Frame>
);

/** Avatar VRX: rectangular receiver with a screen, three buttons, antenna ports. */
const VrxSE = (p: SVGProps<SVGSVGElement>) => (
  <Frame {...p}>
    <rect x="5" y="16" width="32" height="18" rx="2" />
    <rect x="8" y="19" width="16" height="8" rx="1" />
    <circle cx="29" cy="22" r="1" fill="currentColor" />
    <circle cx="33" cy="22" r="1" fill="currentColor" />
    <circle cx="29" cy="26" r="1" fill="currentColor" />
    <circle cx="33" cy="26" r="1" fill="currentColor" />
    <path d="M37 22h4l4-3" />
    <path d="M37 28h4l4 3" />
  </Frame>
);

/** Avatar Relay: antenna tower motif, suggesting signal rebroadcast. */
const AvatarRelay = (p: SVGProps<SVGSVGElement>) => (
  <Frame {...p}>
    <path d="M24 9v30" />
    <path d="M19 39h10" />
    <path d="M17 14a9 9 0 0 0 0 14" />
    <path d="M31 14a9 9 0 0 1 0 14" />
    <path d="M20 18a4 4 0 0 0 0 6" />
    <path d="M28 18a4 4 0 0 1 0 6" />
    <circle cx="24" cy="21" r="1.5" fill="currentColor" />
  </Frame>
);

export default function HardwareIcon({ hardware, ...rest }: Props) {
  switch (hardware) {
    case "AvatarSky":
      return <AvatarSky {...rest} />;
    case "AvatarMiniSky":
      return <AvatarMiniSky {...rest} />;
    case "MoonlightSky":
      return <MoonlightSky {...rest} />;
    case "AvatarGnd":
      return <AvatarGnd {...rest} />;
    case "GogglesX":
      return <GogglesX {...rest} />;
    case "GogglesL":
      return <GogglesL {...rest} />;
    case "VrxSE":
      return <VrxSE {...rest} />;
    case "ReconHd":
      return <ReconHd {...rest} />;
    case "AvatarRelay":
      return <AvatarRelay {...rest} />;
  }
}
