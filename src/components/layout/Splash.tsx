import { PlayMaxSidebarLogo } from "./PlayMaxSidebarLogo";

interface SplashProps {
  visible: boolean;
}

export function Splash({ visible }: SplashProps) {
  if (!visible) return null;

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center app-bg">
      <div className="flex flex-col items-center gap-4">
        <PlayMaxSidebarLogo className="size-20 drop-shadow-2xl drop-shadow-accent/30" />
        <h1 className="text-3xl font-bold tracking-tight">Play Max</h1>
      </div>
    </div>
  );
}

