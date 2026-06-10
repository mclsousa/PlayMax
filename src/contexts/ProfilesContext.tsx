import {
  createContext,
  useCallback,
  useContext,
  useEffect,
  useMemo,
  useRef,
  useState,
  type ReactNode,
} from "react";
import * as api from "../lib/api";
import { useLicense } from "./LicenseContext";
import type { Profile } from "../lib/types";

interface ProfilesContextValue {
  profiles: Profile[];
  activeProfileId: string | null;
  activeProfile: Profile | null;
  loading: boolean;
  error: string | null;
  refresh: (options?: { silent?: boolean }) => Promise<void>;
  setActiveProfile: (profileId: string) => Promise<void>;
  removeProfileOptimistic: (profileId: string) => void;
}

const ProfilesContext = createContext<ProfilesContextValue | null>(null);

export function ProfilesProvider({ children }: { children: ReactNode }) {
  const { license } = useLicense();
  const [profiles, setProfiles] = useState<Profile[]>([]);
  const [activeProfileId, setActiveProfileId] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const wasLicensedRef = useRef(false);
  // Ref keeps `refresh` stable so the mount effect doesn't re-run (and
  // re-fetch) when the profile list itself changes length.
  const hasProfilesRef = useRef(false);

  const refresh = useCallback(async (options?: { silent?: boolean }) => {
    const silent = options?.silent === true;
    if (!silent) {
      setLoading((current) => current || !hasProfilesRef.current);
    }
    setError(null);
    try {
      const data = await api.listProfiles();
      hasProfilesRef.current = data.length > 0;
      setProfiles(data);

      let activeId: string | null = null;
      try {
        activeId = await api.getActiveProfileId();
      } catch {
        // ACL or legacy build — fall back to first profile.
      }

      const resolved =
        activeId && data.some((profile) => profile.id === activeId)
          ? activeId
          : (data[0]?.id ?? null);

      setActiveProfileId(resolved);

      if (resolved && resolved !== activeId) {
        try {
          await api.setActiveProfileId(resolved);
        } catch {
          // Non-fatal; UI still works with in-memory selection.
        }
      }
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setLoading(false);
    }
  }, []);

  const setActiveProfile = useCallback(async (profileId: string) => {
    try {
      await api.setActiveProfileId(profileId);
    } catch {
      // Keep UI responsive even if persist fails.
    }
    setActiveProfileId(profileId);
  }, []);

  const removeProfileOptimistic = useCallback((profileId: string) => {
    setProfiles((current) => {
      const next = current.filter((profile) => profile.id !== profileId);
      hasProfilesRef.current = next.length > 0;
      setActiveProfileId((activeId) => {
        if (activeId !== profileId) {
          return activeId;
        }
        return next[0]?.id ?? null;
      });
      return next;
    });
  }, []);

  useEffect(() => {
    void refresh();
  }, [refresh]);

  useEffect(() => {
    const licensed = license?.valid === true;
    if (licensed && !wasLicensedRef.current) {
      void refresh();
    }
    wasLicensedRef.current = licensed;
  }, [license?.valid, refresh]);

  const activeProfile = useMemo(
    () => profiles.find((profile) => profile.id === activeProfileId) ?? null,
    [profiles, activeProfileId],
  );

  const value = useMemo(
    () => ({
      profiles,
      activeProfileId,
      activeProfile,
      loading,
      error,
      refresh,
      setActiveProfile,
      removeProfileOptimistic,
    }),
    [
      profiles,
      activeProfileId,
      activeProfile,
      loading,
      error,
      refresh,
      setActiveProfile,
      removeProfileOptimistic,
    ],
  );

  return (
    <ProfilesContext.Provider value={value}>{children}</ProfilesContext.Provider>
  );
}

export function useProfilesContext(): ProfilesContextValue {
  const ctx = useContext(ProfilesContext);
  if (!ctx) {
    throw new Error("useProfiles must be used within ProfilesProvider");
  }
  return ctx;
}
