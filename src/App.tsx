import { useEffect } from "react";
import { useStore } from "@/state/store";
import HardwareList from "@/views/HardwareList";
import HistoryView from "@/views/HistoryView";
import SdCardPanel from "@/views/SdCardPanel";
import UpdateWizard from "@/views/UpdateWizard";
import Settings from "@/views/Settings";
import Onboarding from "@/views/Onboarding";
import About from "@/views/About";
import Footer from "@/components/Footer";
import DisclaimerBanner from "@/components/DisclaimerBanner";
import LoadingOverlay from "@/components/LoadingOverlay";

export default function App() {
  const view = useStore((s) => s.view);
  const init = useStore((s) => s.init);
  const onboarded = useStore((s) => s.onboarded);
  const index = useStore((s) => s.index);
  const indexError = useStore((s) => s.indexError);
  const isFetchingIndex = onboarded === true && index === null && !indexError;

  useEffect(() => {
    init();
  }, [init]);

  if (onboarded === null) {
    return (
      <div className="flex h-full items-center justify-center text-brand-muted">
        Loading…
      </div>
    );
  }

  if (!onboarded) {
    return <Onboarding />;
  }

  return (
    <div className="flex h-full flex-col">
      <DisclaimerBanner />
      <main className="flex-1 overflow-auto p-6 space-y-6">
        <SdCardPanel />
        {view.kind === "home" && <HardwareList />}
        {view.kind === "history" && <HistoryView hardware={view.hardware} />}
        {view.kind === "wizard" && (
          <UpdateWizard hardware={view.hardware} version={view.version} />
        )}
        {view.kind === "settings" && <Settings />}
        {view.kind === "about" && <About />}
      </main>
      <Footer />
      {isFetchingIndex && (
        <LoadingOverlay message="Fetching firmware index from walksnail.app…" />
      )}
    </div>
  );
}
