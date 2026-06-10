import { useEffect, useState } from "react";

import { Button } from "../ui/Button";
import { Input } from "../ui/Input";
import * as api from "../../lib/api";
import type { Profile } from "../../lib/types";

interface EditProfileDialogProps {
  profile: Profile;
  open: boolean;
  onClose: () => void;
  onSaved: () => void;
}

export function EditProfileDialog({
  profile,
  open,
  onClose,
  onSaved,
}: EditProfileDialogProps) {
  const isXtream = profile.type === "xtream";
  const [name, setName] = useState(profile.name);
  const [url, setUrl] = useState(profile.url ?? "");
  const [filePath, setFilePath] = useState(profile.filePath ?? "");
  const [username, setUsername] = useState(profile.username ?? "");
  const [password, setPassword] = useState("");
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (!open) return;
    setName(profile.name);
    setUrl(profile.url ?? "");
    setFilePath(profile.filePath ?? "");
    setUsername(profile.username ?? "");
    setPassword("");
    setError(null);
  }, [open, profile]);

  if (!open) return null;

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

  const handleSave = async () => {
    setSaving(true);
    setError(null);
    try {
      if (isXtream) {
        await api.updateXtreamProfile(profile.id, name, url, username, password);
      } else {
        await api.updateM3uProfile(
          profile.id,
          name,
          url.trim() || undefined,
          filePath.trim() || undefined,
        );
      }
      onSaved();
      onClose();
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setSaving(false);
    }
  };

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/70 p-4">
      <div className="w-full max-w-lg space-y-4 rounded-2xl border border-base-800 bg-base-900 p-5">
        <div>
          <h2 className="text-lg font-semibold">Editar lista</h2>
          <p className="text-sm text-text-muted">
            Atualize URL ou credenciais. Depois de salvar, sincronize novamente.
          </p>
        </div>

        <Input label="Nome" value={name} onChange={(event) => setName(event.target.value)} />

        {isXtream ? (
          <>
            <Input label="URL do servidor" value={url} onChange={(event) => setUrl(event.target.value)} />
            <Input
              label="Usuário"
              value={username}
              onChange={(event) => setUsername(event.target.value)}
            />
            <Input
              label="Senha"
              type="password"
              value={password}
              onChange={(event) => setPassword(event.target.value)}
              placeholder="Nova senha"
            />
          </>
        ) : (
          <>
            <Input label="URL M3U" value={url} onChange={(event) => setUrl(event.target.value)} />
            <div className="flex items-end gap-2">
              <div className="flex-1">
                <Input
                  label="Arquivo local"
                  value={filePath}
                  onChange={(event) => setFilePath(event.target.value)}
                  readOnly
                />
              </div>
              <Button type="button" variant="secondary" onClick={() => void handlePickFile()}>
                Arquivo
              </Button>
            </div>
          </>
        )}

        {error ? (
          <p className="rounded-xl border border-red-500/30 bg-red-500/10 px-3 py-2 text-sm text-red-200">
            {error}
          </p>
        ) : null}

        <div className="flex justify-end gap-2">
          <Button type="button" variant="secondary" onClick={onClose}>
            Cancelar
          </Button>
          <Button type="button" onClick={() => void handleSave()} disabled={saving}>
            {saving ? "Salvando..." : "Salvar"}
          </Button>
        </div>
      </div>
    </div>
  );
}
