import { listen } from "@tauri-apps/api/event";
import { useCallback, useEffect, useState } from "react";
import { useNavigate } from "react-router-dom";
import { Button } from "../components/ui/Button";
import { Input } from "../components/ui/Input";
import { TopBar } from "../components/layout/TopBar";
import { EditProfileDialog } from "../components/settings/EditProfileDialog";
import * as api from "../lib/api";
import { clearCategoryCache } from "../lib/categoryCache";
import { useLicense } from "../contexts/LicenseContext";
import { useParentalControl } from "../contexts/ParentalControlContext";
import { useActiveProfile } from "../hooks/useActiveProfile";
import { useProfiles } from "../hooks/useProfiles";
import type { AppSettings, Profile, SyncProgress } from "../lib/types";

function formatSyncTime(timestamp: number | null | undefined): string {
  if (!timestamp) return "Nunca";
  return new Date(timestamp * 1000).toLocaleString("pt-BR", {
    dateStyle: "short",
    timeStyle: "short",
  });
}

export function SettingsPage() {
  const navigate = useNavigate();
  const { license, logout } = useLicense();
  const { profiles, loading, refresh, removeProfileOptimistic } = useProfiles();
  const { activeProfile, setActiveProfile } = useActiveProfile();
  const { lock, refresh: refreshParental, unlocked } = useParentalControl();
  const [syncingId, setSyncingId] = useState<string | null>(null);
  const [progress, setProgress] = useState<SyncProgress | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [settings, setSettings] = useState<AppSettings | null>(null);
  const [settingsLoading, setSettingsLoading] = useState(true);
  const [editingProfile, setEditingProfile] = useState<Profile | null>(null);
  const [parentalPin, setParentalPin] = useState("");
  const [parentalSaving, setParentalSaving] = useState(false);

  const loadSettings = useCallback(async () => {
    setSettingsLoading(true);
    try {
      const data = await api.getAppSettings();
      setSettings(data);
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setSettingsLoading(false);
    }
  }, []);

  useEffect(() => {
    void loadSettings();
  }, [loadSettings]);

  useEffect(() => {
    const unlisten = listen<SyncProgress>("sync-progress", (event) => {
      setProgress(event.payload);
      if (event.payload.percent >= 100) {
        setSyncingId(null);
        void refresh();
        void loadSettings();
      }
    });
    const unlistenError = listen<string>("sync-error", (event) => {
      setError(event.payload);
      setSyncingId(null);
    });
    return () => {
      void unlisten.then((fn) => fn());
      void unlistenError.then((fn) => fn());
    };
  }, [refresh, loadSettings]);

  const handleSync = async (profileId: string) => {
    setError(null);
    setSyncingId(profileId);
    setProgress(null);
    try {
      await api.syncProfile(profileId);
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
      setSyncingId(null);
    }
  };

  const handleRemove = (profileId: string) => {
    setError(null);
    const wasLast = profiles.length <= 1;
    removeProfileOptimistic(profileId);
    clearCategoryCache(profileId);
    void (async () => {
      try {
        await api.removeProfile(profileId);
        await refresh();
        if (wasLast) {
          navigate("/onboarding", { replace: true });
        }
      } catch (err) {
        await refresh();
        setError(err instanceof Error ? err.message : String(err));
      }
    })();
  };

  const handleAutoSyncToggle = async (enabled: boolean) => {
    setError(null);
    try {
      const updated = await api.updateAppSettings({ autoSyncEnabled: enabled });
      setSettings(updated);
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    }
  };

  const handleSaveParentalPin = async () => {
    setError(null);
    setParentalSaving(true);
    try {
      const updated = await api.updateAppSettings({ parentalPin });
      setSettings(updated);
      setParentalPin("");
      await refreshParental();
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setParentalSaving(false);
    }
  };

  const handleParentalToggle = async (enabled: boolean) => {
    setError(null);
    try {
      const updated = await api.updateAppSettings({ parentalControlEnabled: enabled });
      setSettings(updated);
      await refreshParental();
      if (!enabled) {
        lock();
      }
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    }
  };

  const handleLogout = async () => {
    setError(null);
    try {
      await logout();
      navigate("/activate", { replace: true });
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    }
  };

  const lastSyncLabel = formatSyncTime(
    activeProfile?.lastSync ?? settings?.lastBackgroundSync ?? null,
  );

  return (
    <div className="flex h-full flex-col overflow-hidden app-bg">
      <TopBar showSearch={false} />
      <div className="scrollbar-thin flex-1 overflow-y-auto bg-base-950 px-8 pb-8">
        <div className="mx-auto max-w-3xl space-y-6">
          <div>
            <h1 className="text-2xl font-bold">Configurações</h1>
            <p className="text-text-secondary">Gerencie licença, listas IPTV e preferências.</p>
          </div>

          <section className="rounded-2xl border border-base-800 bg-base-850 p-5">
            <h2 className="mb-4 font-semibold">Licença</h2>
            <div className="space-y-2 text-sm">
              <p>
                Chave:{" "}
                <span className="font-mono text-text-primary">
                  {license?.licenseKey ?? "—"}
                </span>
              </p>
              <p>
                Status:{" "}
                <span className="text-text-primary">{license?.status ?? "—"}</span>
              </p>
              {license?.expiresAt ? (
                <p>
                  Válida até:{" "}
                  <span className="text-text-primary">
                    {formatSyncTime(license.expiresAt)}
                  </span>
                </p>
              ) : null}
            </div>
            <div className="mt-4">
              <Button size="sm" variant="secondary" onClick={() => void handleLogout()}>
                Sair da licença
              </Button>
              <p className="mt-2 text-xs text-text-muted">
                Encerra a sessão da licença neste computador. Suas listas IPTV permanecem salvas.
              </p>
            </div>
          </section>

          <section className="rounded-2xl border border-base-800 bg-base-850 p-5">
            <h2 className="mb-4 font-semibold">Sincronização</h2>
            {settingsLoading ? (
              <p className="text-sm text-text-secondary">Carregando...</p>
            ) : (
              <div className="space-y-4">
                <label className="flex cursor-pointer items-center justify-between gap-4 rounded-xl border border-base-800 bg-base-900/60 p-4">
                  <div>
                    <p className="font-medium">Atualizar catálogo automaticamente</p>
                    <p className="text-xs text-text-muted">
                      Sincroniza a lista ativa a cada 30 minutos enquanto o app estiver aberto.
                    </p>
                  </div>
                  <input
                    type="checkbox"
                    className="h-5 w-5 accent-accent"
                    checked={settings?.autoSyncEnabled ?? false}
                    onChange={(event) => void handleAutoSyncToggle(event.target.checked)}
                  />
                </label>
                <p className="text-sm text-text-secondary">
                  Última sincronização: <span className="text-text-primary">{lastSyncLabel}</span>
                  {activeProfile ? (
                    <span className="text-text-muted"> · {activeProfile.name}</span>
                  ) : null}
                </p>
              </div>
            )}
          </section>

          <section className="rounded-2xl border border-base-800 bg-base-850 p-5">
            <h2 className="mb-4 font-semibold">Controle parental</h2>
            {settingsLoading ? (
              <p className="text-sm text-text-secondary">Carregando...</p>
            ) : (
              <div className="space-y-4">
                <p className="text-sm text-text-secondary">
                  Bloqueia categorias adultas (+18, XXX, Adulto). O desbloqueio
                  vale por 30 minutos após digitar o PIN.
                </p>
                <div className="flex flex-wrap items-end gap-3">
                  <div className="min-w-[160px]">
                    <label className="mb-1 block text-xs text-text-muted">
                      PIN de 4 dígitos
                    </label>
                    <Input
                      type="password"
                      inputMode="numeric"
                      maxLength={4}
                      value={parentalPin}
                      onChange={(event) =>
                        setParentalPin(
                          event.target.value.replace(/\D/g, "").slice(0, 4),
                        )
                      }
                      placeholder="0000"
                      className="tracking-[0.35em]"
                    />
                  </div>
                  <Button
                    size="sm"
                    variant="secondary"
                    disabled={parentalPin.length !== 4 || parentalSaving}
                    onClick={() => void handleSaveParentalPin()}
                  >
                    {parentalSaving ? "Salvando..." : "Salvar PIN"}
                  </Button>
                </div>
                <label className="flex cursor-pointer items-center justify-between gap-4 rounded-xl border border-base-800 bg-base-900/60 p-4">
                  <div>
                    <p className="font-medium">Ativar controle parental</p>
                    <p className="text-xs text-text-muted">
                      {settings?.parentalPinSet
                        ? "Categorias adultas exigem PIN."
                        : "Salve um PIN antes de ativar."}
                    </p>
                  </div>
                  <input
                    type="checkbox"
                    className="h-5 w-5 accent-accent"
                    checked={settings?.parentalControlEnabled ?? false}
                    disabled={!settings?.parentalPinSet}
                    onChange={(event) =>
                      void handleParentalToggle(event.target.checked)
                    }
                  />
                </label>
                {settings?.parentalControlEnabled && unlocked ? (
                  <Button size="sm" variant="secondary" onClick={lock}>
                    Bloquear conteúdo adulto agora
                  </Button>
                ) : null}
              </div>
            )}
          </section>

          <section className="rounded-2xl border border-base-800 bg-base-850 p-5">
            <div className="mb-4 flex flex-wrap items-center justify-between gap-3">
              <div>
                <h2 className="font-semibold">Listas IPTV</h2>
                {activeProfile ? (
                  <p className="mt-1 text-xs text-text-muted">
                    Lista ativa: <span className="text-text-primary">{activeProfile.name}</span>
                  </p>
                ) : null}
              </div>
              <Button
                size="sm"
                variant="secondary"
                onClick={() => navigate("/onboarding", { state: { addList: true } })}
              >
                Adicionar lista
              </Button>
            </div>
            {loading ? (
              <p className="text-sm text-text-secondary">Carregando...</p>
            ) : profiles.length === 0 ? (
              <p className="text-sm text-text-secondary">Nenhuma lista configurada.</p>
            ) : (
              <div className="flex flex-col gap-3">
                {profiles.map((profile) => {
                  const isActive = profile.id === activeProfile?.id;
                  return (
                  <div
                    key={profile.id}
                    className={`flex flex-wrap items-center justify-between gap-3 rounded-xl border p-4 ${
                      isActive
                        ? "border-accent/40 bg-accent/5"
                        : "border-base-800 bg-base-900/60"
                    }`}
                  >
                    <div>
                      <p className="font-medium">
                        {profile.name}
                        {isActive ? (
                          <span className="ml-2 rounded-full bg-accent/15 px-2 py-0.5 text-[10px] font-semibold uppercase tracking-wide text-accent">
                            Ativa
                          </span>
                        ) : null}
                      </p>
                      <p className="text-xs text-text-muted">
                        {profile.url ?? profile.filePath ?? "Sem fonte"}
                      </p>
                      {profile.lastSync ? (
                        <p className="text-xs text-text-muted">
                          Sincronizado em {formatSyncTime(profile.lastSync)}
                        </p>
                      ) : null}
                    </div>
                    <div className="flex flex-wrap gap-2">
                      {!isActive ? (
                        <Button
                          size="sm"
                          onClick={() => void setActiveProfile(profile.id)}
                        >
                          Usar esta lista
                        </Button>
                      ) : null}
                      <Button
                        size="sm"
                        variant="secondary"
                        onClick={() => setEditingProfile(profile)}
                      >
                        Editar
                      </Button>
                      <Button
                        size="sm"
                        variant="secondary"
                        disabled={syncingId === profile.id}
                        onClick={() => void handleSync(profile.id)}
                      >
                        {syncingId === profile.id
                          ? progress
                            ? `Atualizando ${progress.percent}%`
                            : "Atualizando..."
                          : "Atualizar"}
                      </Button>
                      <Button
                        size="sm"
                        variant="danger"
                        onClick={() => void handleRemove(profile.id)}
                      >
                        Remover
                      </Button>
                    </div>
                  </div>
                );
                })}
              </div>
            )}
          </section>

          {error && (
            <p className="rounded-xl border border-red-500/30 bg-red-500/10 px-3 py-2 text-sm text-red-200">
              {error}
            </p>
          )}
        </div>
      </div>

      {editingProfile ? (
        <EditProfileDialog
          profile={editingProfile}
          open
          onClose={() => setEditingProfile(null)}
          onSaved={() => void refresh()}
        />
      ) : null}
    </div>
  );
}
