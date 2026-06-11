import { useState } from "react";
import { useNavigate } from "react-router-dom";
import { openUrl } from "@tauri-apps/plugin-opener";

import { Button } from "../components/ui/Button";
import { Input } from "../components/ui/Input";
import { PlayMaxSidebarLogo } from "../components/layout/PlayMaxSidebarLogo";
import { useLicense } from "../contexts/LicenseContext";
import { useProfiles } from "../hooks/useProfiles";

export function ActivationPage() {
  const navigate = useNavigate();
  const { activate, error, loading } = useLicense();
  const { refresh: refreshProfiles } = useProfiles();
  const [licenseKey, setLicenseKey] = useState("");

  const handleSubmit = async (event: React.FormEvent) => {
    event.preventDefault();
    try {
      await activate(licenseKey.trim().toUpperCase());
      await refreshProfiles();
      navigate("/", { replace: true });
    } catch {
      // error shown via context
    }
  };

  return (
    <div className="flex min-h-full flex-col items-center justify-center bg-base-950 px-6 py-12">
      <div className="w-full max-w-md space-y-6 rounded-2xl border border-base-800 bg-base-900/70 p-6">
        <div className="space-y-2 text-center">
          <PlayMaxSidebarLogo className="mx-auto mb-2 size-16 drop-shadow-lg drop-shadow-accent/25" />
          <h1 className="text-2xl font-bold text-text-primary">Ativar Play Max</h1>
          <p className="text-sm text-text-secondary">
            Informe a chave <span className="font-mono text-text-primary">PLAY-XXXX-XXXX-XXXX</span>{" "}
            enviada pelo seu provedor.
          </p>
        </div>

        <form className="space-y-4" onSubmit={(event) => void handleSubmit(event)}>
          <Input
            label="Chave de licença"
            value={licenseKey}
            onChange={(event) => setLicenseKey(event.target.value.toUpperCase())}
            placeholder="PLAY-AB12-CD34-EF56"
            autoComplete="off"
            spellCheck={false}
          />

          {error ? (
            <p className="rounded-xl border border-red-500/30 bg-red-500/10 px-3 py-2 text-sm text-red-200">
              {error}
            </p>
          ) : null}

          <Button type="submit" className="w-full" disabled={loading || !licenseKey.trim()}>
            {loading ? "Validando..." : "Ativar licença"}
          </Button>
        </form>

        <p className="text-center text-sm text-text-secondary">
          Ainda não tem licença?{" "}
          <button
            type="button"
            className="font-semibold text-accent underline-offset-2 hover:underline"
            onClick={() => void openUrl("https://playmx.com.br")}
          >
            Compre em playmx.com.br
          </button>
        </p>

        <p className="text-center text-xs text-text-muted">
          Cada licença funciona em <strong>1 computador</strong>. Precisa renovar? Fale com seu
          provedor.
        </p>
      </div>
    </div>
  );
}
