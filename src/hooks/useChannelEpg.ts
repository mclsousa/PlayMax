import { useEffect, useState } from "react";
import { getChannelEpg } from "../lib/api";
import type { ChannelEpg } from "../lib/types";

export function useChannelEpg(
  profileId: string | null,
  channelId: string | null,
  enabled = true,
  limit = 4,
) {
  const [epg, setEpg] = useState<ChannelEpg | null>(null);
  const [loading, setLoading] = useState(false);

  useEffect(() => {
    if (!enabled || !profileId || !channelId) {
      setEpg(null);
      setLoading(false);
      return;
    }

    let cancelled = false;
    setLoading(true);

    void getChannelEpg(profileId, channelId, limit)
      .then((result) => {
        if (!cancelled) setEpg(result);
      })
      .catch(() => {
        if (!cancelled) {
          setEpg({
            available: false,
            unavailableReason: "EPG indisponível",
            programs: [],
          });
        }
      })
      .finally(() => {
        if (!cancelled) setLoading(false);
      });

    return () => {
      cancelled = true;
    };
  }, [profileId, channelId, enabled, limit]);

  return { epg, loading };
}
