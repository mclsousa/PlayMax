import { lazy, Suspense, useEffect, useState } from "react";
import { BrowserRouter, Navigate, Route, Routes } from "react-router-dom";
import { LicenseProvider, useLicense } from "./contexts/LicenseContext";
import { ParentalControlProvider } from "./contexts/ParentalControlContext";
import { ProfilesProvider } from "./contexts/ProfilesContext";
import { FavoritesProvider } from "./contexts/FavoritesContext";
import { PlayerRouteLayout } from "./components/player/PlayerRouteLayout";
import { AppShell } from "./components/layout/AppShell";
import { PlayerProvider } from "./contexts/PlayerContext";
import { Splash } from "./components/layout/Splash";
import { useProfiles } from "./hooks/useProfiles";
import { useSyncCategoryCacheInvalidation } from "./hooks/useSyncCategoryCacheInvalidation";
import { ActivationPage } from "./pages/ActivationPage";
import { HomePage } from "./pages/HomePage";
import "./styles/globals.css";

const MovieDetailPage = lazy(() =>
  import("./pages/MovieDetailPage").then((module) => ({
    default: module.MovieDetailPage,
  })),
);
const SeriesDetailPage = lazy(() =>
  import("./pages/SeriesDetailPage").then((module) => ({
    default: module.SeriesDetailPage,
  })),
);

const LivePage = lazy(() =>
  import("./pages/LivePage").then((module) => ({ default: module.LivePage })),
);
const MoviesPage = lazy(() =>
  import("./pages/MoviesPage").then((module) => ({ default: module.MoviesPage })),
);
const SeriesPage = lazy(() =>
  import("./pages/SeriesPage").then((module) => ({ default: module.SeriesPage })),
);
const MyListPage = lazy(() =>
  import("./pages/MyListPage").then((module) => ({ default: module.MyListPage })),
);
const SearchPage = lazy(() =>
  import("./pages/SearchPage").then((module) => ({ default: module.SearchPage })),
);
const SettingsPage = lazy(() =>
  import("./pages/SettingsPage").then((module) => ({ default: module.SettingsPage })),
);
const OnboardingPage = lazy(() =>
  import("./pages/OnboardingPage").then((module) => ({
    default: module.OnboardingPage,
  })),
);

function RouteFallback() {
  return (
    <div className="flex min-h-full flex-1 items-center justify-center bg-base-950 text-sm text-text-muted">
      Carregando...
    </div>
  );
}

function RootRedirect() {
  const { profiles, loading } = useProfiles();
  if (loading) {
    return <RouteFallback />;
  }
  return (
    <Navigate to={profiles.length > 0 ? "/home" : "/onboarding"} replace />
  );
}

function LicensedRoutes() {
  const { license, loading } = useLicense();

  if (loading) {
    return <RouteFallback />;
  }

  if (!license?.valid) {
    return (
      <Routes>
        <Route path="/activate" element={<ActivationPage />} />
        <Route path="*" element={<Navigate to="/activate" replace />} />
      </Routes>
    );
  }

  return (
    <Routes>
      <Route path="/activate" element={<Navigate to="/" replace />} />
      <Route path="/onboarding" element={<OnboardingPage />} />
      <Route element={<AppShell />}>
        <Route path="/" element={<RootRedirect />} />
        <Route path="/home" element={<HomePage />} />
        <Route element={<PlayerProvider />}>
          <Route element={<PlayerRouteLayout />}>
            <Route path="/live" element={<LivePage />} />
            <Route path="/movies" element={<MoviesPage />} />
            <Route path="/series" element={<SeriesPage />} />
          </Route>
        </Route>
        <Route path="/movies/:id" element={<MovieDetailPage />} />
        <Route path="/series/:id" element={<SeriesDetailPage />} />
        <Route path="/list" element={<MyListPage />} />
        <Route path="/search" element={<SearchPage />} />
        <Route path="/settings" element={<SettingsPage />} />
      </Route>
      <Route path="*" element={<Navigate to="/" replace />} />
    </Routes>
  );
}

export default function App() {
  const [showSplash, setShowSplash] = useState(true);
  useSyncCategoryCacheInvalidation();

  useEffect(() => {
    // Just long enough to cover the first paint; real loading states take
    // over from here, so a longer fixed splash only delays the app.
    const timer = window.setTimeout(() => setShowSplash(false), 350);
    return () => window.clearTimeout(timer);
  }, []);

  return (
    <>
      <Splash visible={showSplash} />
      <BrowserRouter>
        <LicenseProvider>
          <ProfilesProvider>
            <ParentalControlProvider>
              <FavoritesProvider>
                <div className="flex h-full min-h-0 flex-col overflow-hidden app-bg">
                  <Suspense fallback={<RouteFallback />}>
                    <LicensedRoutes />
                  </Suspense>
                </div>
              </FavoritesProvider>
            </ParentalControlProvider>
          </ProfilesProvider>
        </LicenseProvider>
      </BrowserRouter>
    </>
  );
}
