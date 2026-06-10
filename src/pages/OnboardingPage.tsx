import { listen } from "@tauri-apps/api/event";
import { useEffect, useState } from "react";
import { useLocation, useNavigate } from "react-router-dom";
import { Button } from "../components/ui/Button";
import { Input } from "../components/ui/Input";
import { PlayMaxSidebarLogo } from "../components/layout/PlayMaxSidebarLogo";
import * as api from "../lib/api";
import { useProfiles } from "../hooks/useProfiles";
import { runningInTauri } from "../lib/tauri";
import type { SyncProgress } from "../lib/types";

type SourceType = "m3u" | "xtream";

export function OnboardingPage() {
  const navigate = useNavigate();
  const location = useLocation();
  const addingAnotherList = Boolean(
    (location.state as { addList?: boolean } | null)?.addList,
  );
  const { profiles, loading: profilesLoading, refresh: refreshProfiles, setActiveProfile } =
    useProfiles();
  const [sourceType, setSourceType] = useState<SourceType>("m3u");
  const [name, setName] = useState("Minha Lista");
  const [url, setUrl] = useState("");
  const [username, setUsername] = useState("");
  const [password, setPassword] = useState("");
  const [filePath, setFilePath] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [syncing, setSyncing] = useState(false);
  const [progress, setProgress] = useState<SyncProgress | null>(null);

  useEffect(() => {
    const unlistenProgress = listen<SyncProgress>("sync-progress", (event) => {
      setProgress(event.payload);
    });
    const unlistenError = listen<string>("sync-error", (event) => {
      setError(event.payload);
      setSyncing(false);
    });
    return () => {
      void unlistenProgress.then((fn) => fn());
      void unlistenError.then((fn) => fn());
    };
  }, []);

  useEffect(() => {
    if (addingAnotherList || profilesLoading) {
      return;
    }
    if (profiles.length > 0) {
      navigate("/home", { replace: true });
    }
  }, [addingAnotherList, navigate, profiles.length, profilesLoading]);

  const handlePickFile = async () => {
    setError(null);
    try {
      const path = await api.pickM3uFile();
      if (path) {
        setFilePath(path);
        setUrl("");
      }
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    }
  };

  const canSubmit =
    sourceType === "m3u"
      ? !!url.trim() || !!filePath
      : !!url.trim() && !!username.trim() && !!password.trim();

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    setError(null);
    setSyncing(true);
    setProgress(null);

    try {
      const profile =
        sourceType === "m3u"
          ? await api.addM3uProfile(
              name,
              url.trim() || undefined,
              filePath ?? undefined,
            )
          : await api.addXtreamProfile(
              name,
              url.trim(),
              username.trim(),
              password,
            );

      await setActiveProfile(profile.id);
      void refreshProfiles({ silent: true });
      void api.syncProfile(profile.id);
      navigate("/home", { replace: true });
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setSyncing(false);
    }
  };

  return (
    <div className="flex h-full min-h-0 flex-col overflow-hidden app-bg">
      <div className="flex min-h-0 flex-1 items-center justify-center overflow-hidden p-6">
        <div className="w-full max-w-lg rounded-2xl border border-base-800 bg-base-900/80 p-6 shadow-2xl backdrop-blur-sm">
        <div className="mb-6 text-center">
          <PlayMaxSidebarLogo className="mx-auto mb-4 size-16 drop-shadow-lg drop-shadow-accent/25" />
          <h1 className="text-2xl font-bold">Bem-vindo ao Play Max</h1>
          <p className="mt-2 text-text-secondary">
            Adicione sua lista M3U ou credenciais Xtream Codes para começar a
            assistir canais ao vivo, filmes e séries.
          </p>
        </div>

        {!runningInTauri() && (
          <div className="mb-4 rounded-lg border border-amber-500/40 bg-amber-500/10 px-3 py-2 text-sm text-amber-100">
            Você está no preview do navegador. Para importar listas, inicie o app com{" "}
            <code className="rounded bg-base-800 px-1">npm run tauri:dev</code>.
          </div>
        )}

        <form className="flex flex-col gap-4" onSubmit={handleSubmit}>
          <div className="flex rounded-full border border-base-700 bg-base-950 p-0.5 ring-1 ring-base-700/40">
            <button
              type="button"
              className={`flex-1 rounded-full px-3 py-2 text-sm font-medium transition-all duration-250 ${
                sourceType === "m3u"
                  ? "bg-base-800 text-text-primary shadow-sm"
                  : "text-text-secondary hover:text-text-primary"
              }`}
              onClick={() => setSourceType("m3u")}
            >
              M3U
            </button>
            <button
              type="button"
              className={`flex-1 rounded-full px-3 py-2 text-sm font-medium transition-all duration-250 ${
                sourceType === "xtream"
                  ? "bg-base-800 text-text-primary shadow-sm"
                  : "text-text-secondary hover:text-text-primary"
              }`}
              onClick={() => setSourceType("xtream")}
            >
              Xtream
            </button>
          </div>

          <Input
            label="Nome da lista"
            value={name}
            onChange={(e) => setName(e.target.value)}
            required
          />

          {sourceType === "m3u" ? (
            <>
              <Input
                label="URL da lista M3U"
                placeholder="https://exemplo.com/lista.m3u"
                value={url}
                onChange={(e) => {
                  setUrl(e.target.value);
                  setFilePath(null);
                }}
                disabled={!!filePath}
              />

              <div className="flex items-center gap-3 text-sm text-text-secondary">
                <span className="h-px flex-1 bg-base-700" />
                ou
                <span className="h-px flex-1 bg-base-700" />
              </div>

              <Button type="button" variant="secondary" onClick={handlePickFile}>
                Escolher arquivo M3U
              </Button>
              {filePath && (
                <p className="rounded-lg bg-base-800 px-3 py-2 text-xs text-text-secondary">
                  Arquivo: {filePath}
                </p>
              )}
            </>
          ) : (
            <>
              <Input
                label="URL do servidor"
                placeholder="http://exemplo.com:8080"
                value={url}
                onChange={(e) => setUrl(e.target.value)}
                required
              />
              <Input
                label="Usuário"
                value={username}
                onChange={(e) => setUsername(e.target.value)}
                required
              />
              <Input
                label="Senha"
                type="password"
                value={password}
                onChange={(e) => setPassword(e.target.value)}
                required
              />
            </>
          )}

          {syncing && progress && (
            <div className="rounded-lg border border-base-700 bg-base-950 p-4">
              <div className="mb-2 flex justify-between text-xs text-text-secondary">
                <span>{progress.message}</span>
                <span>{Math.round(progress.percent)}%</span>
              </div>
              <div className="h-2 overflow-hidden rounded-full bg-base-800">
                <div
                  className="h-full rounded-full bg-accent transition-all duration-250"
                  style={{ width: `${progress.percent}%` }}
                />
              </div>
            </div>
          )}

          {error && (
            <p className="rounded-lg border border-red-500/30 bg-red-500/10 px-3 py-2 text-sm text-red-200">
              {error}
            </p>
          )}

          <Button type="submit" disabled={syncing || !canSubmit}>
            {syncing ? "Adicionando..." : "Adicionar lista"}
          </Button>
        </form>
        </div>
      </div>
    </div>
  );
}
