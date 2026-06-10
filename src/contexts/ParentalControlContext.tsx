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
import { isAdultCategory } from "../lib/parentalControl";
import { ParentalPinModal } from "../components/parental/ParentalPinModal";

interface ParentalControlContextValue {
  enabled: boolean;
  pinSet: boolean;
  unlocked: boolean;
  loading: boolean;
  isCategoryBlocked: (category: string) => boolean;
  requestAccess: (onGranted: () => void) => void;
  lock: () => void;
  refresh: () => Promise<void>;
}

const ParentalControlContext =
  createContext<ParentalControlContextValue | null>(null);

const UNLOCK_TTL_MS = 30 * 60 * 1000;

export function ParentalControlProvider({ children }: { children: ReactNode }) {
  const [enabled, setEnabled] = useState(false);
  const [pinSet, setPinSet] = useState(false);
  const [unlockedUntil, setUnlockedUntil] = useState<number | null>(null);
  const [loading, setLoading] = useState(true);
  const [modalOpen, setModalOpen] = useState(false);
  const [pendingAction, setPendingAction] = useState<(() => void) | null>(null);
  const [modalError, setModalError] = useState<string | null>(null);

  const refresh = useCallback(async () => {
    setLoading(true);
    try {
      const settings = await api.getAppSettings();
      setEnabled(settings.parentalControlEnabled);
      setPinSet(settings.parentalPinSet);
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    void refresh();
  }, [refresh]);

  const unlocked =
    unlockedUntil != null && unlockedUntil > Date.now();

  const isCategoryBlocked = useCallback(
    (category: string) => {
      if (!enabled || unlocked) return false;
      if (category === "all") return false;
      return isAdultCategory(category);
    },
    [enabled, unlocked],
  );

  const requestAccess = useCallback(
    (onGranted: () => void) => {
      if (!enabled || unlocked) {
        onGranted();
        return;
      }
      setModalError(null);
      setPendingAction(() => onGranted);
      setModalOpen(true);
    },
    [enabled, unlocked],
  );

  const lock = useCallback(() => {
    setUnlockedUntil(null);
  }, []);

  const handleVerifyPin = useCallback(
    async (pin: string) => {
      setModalError(null);
      const ok = await api.verifyParentalPin(pin);
      if (!ok) {
        setModalError("PIN incorreto.");
        return;
      }
      setUnlockedUntil(Date.now() + UNLOCK_TTL_MS);
      setModalOpen(false);
      const action = pendingAction;
      setPendingAction(null);
      action?.();
    },
    [pendingAction],
  );

  const value = useMemo(
    () => ({
      enabled,
      pinSet,
      unlocked,
      loading,
      isCategoryBlocked,
      requestAccess,
      lock,
      refresh,
    }),
    [
      enabled,
      pinSet,
      unlocked,
      loading,
      isCategoryBlocked,
      requestAccess,
      lock,
      refresh,
    ],
  );

  return (
    <ParentalControlContext.Provider value={value}>
      {children}
      <ParentalPinModal
        open={modalOpen}
        title="Conteúdo restrito"
        description="Digite o PIN parental para acessar esta categoria."
        error={modalError}
        onClose={() => {
          setModalOpen(false);
          setPendingAction(null);
          setModalError(null);
        }}
        onSubmit={handleVerifyPin}
      />
    </ParentalControlContext.Provider>
  );
}

export function useParentalControl(): ParentalControlContextValue {
  const ctx = useContext(ParentalControlContext);
  if (!ctx) {
    throw new Error(
      "useParentalControl must be used within ParentalControlProvider",
    );
  }
  return ctx;
}
