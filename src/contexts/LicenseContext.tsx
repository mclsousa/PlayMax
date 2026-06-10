import {
  createContext,
  useCallback,
  useContext,
  useEffect,
  useMemo,
  useState,
  type ReactNode,
} from "react";

import * as api from "../lib/api";
import { LICENSE_API_URL } from "../lib/licenseConfig";
import { runningInTauri } from "../lib/tauri";
import type { LicenseState } from "../lib/types";

interface LicenseContextValue {
  license: LicenseState | null;
  loading: boolean;
  error: string | null;
  refresh: () => Promise<void>;
  activate: (licenseKey: string) => Promise<void>;
  logout: () => Promise<void>;
}

const LicenseContext = createContext<LicenseContextValue | null>(null);

const DEV_BYPASS = import.meta.env.DEV && !runningInTauri();

export function LicenseProvider({ children }: { children: ReactNode }) {
  const [license, setLicense] = useState<LicenseState | null>(
    DEV_BYPASS
      ? { valid: true, status: "active", licenseKey: "DEV-BYPASS" }
      : null,
  );
  const [loading, setLoading] = useState(!DEV_BYPASS);
  const [error, setError] = useState<string | null>(null);

  const refresh = useCallback(async () => {
    if (DEV_BYPASS) {
      setLicense({ valid: true, status: "active", licenseKey: "DEV-BYPASS" });
      setLoading(false);
      return;
    }

    setError(null);
    let local: LicenseState | null = null;
    try {
      local = await api.getLicenseState();
      setLicense(local);
      // Locally valid (within the offline grace period) or no key at all:
      // render the app right away and let the network validation below only
      // refine the state in background. Only a key that lost local validity
      // still blocks, so the activation screen never flashes by mistake.
      if (local.valid || !local.licenseKey) {
        setLoading(false);
      }
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
      setLicense({ valid: false, message: "Falha ao validar licença." });
      setLoading(false);
      return;
    }

    if (!local.licenseKey) {
      return;
    }

    try {
      const validated = await api.validateLicense(LICENSE_API_URL);
      setLicense(validated);
    } catch (err) {
      // Offline or license server unreachable — keep the local state.
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setLoading(false);
    }
  }, []);

  const activate = useCallback(async (licenseKey: string) => {
    setLoading(true);
    setError(null);
    try {
      const state = await api.activateLicense(LICENSE_API_URL, licenseKey);
      setLicense(state);
      if (!state.valid) {
        throw new Error(state.message ?? "Licença inválida.");
      }
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
      throw err;
    } finally {
      setLoading(false);
    }
  }, []);

  const logout = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      await api.clearLicense();
      setLicense({
        valid: false,
        licenseKey: null,
        status: null,
        expiresAt: null,
        lastValidatedAt: null,
        message: "Sessão encerrada.",
      });
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
      throw err;
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    void refresh();
  }, [refresh]);

  const value = useMemo(
    () => ({ license, loading, error, refresh, activate, logout }),
    [license, loading, error, refresh, activate, logout],
  );

  return <LicenseContext.Provider value={value}>{children}</LicenseContext.Provider>;
}

export function useLicense(): LicenseContextValue {
  const ctx = useContext(LicenseContext);
  if (!ctx) {
    throw new Error("useLicense must be used within LicenseProvider");
  }
  return ctx;
}
