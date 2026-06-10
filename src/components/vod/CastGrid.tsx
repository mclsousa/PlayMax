import { useState } from "react";
import type { CastMember } from "../../lib/types";

interface CastGridProps {
  cast: CastMember[];
}

function CastAvatar({ member }: { member: CastMember }) {
  const [failed, setFailed] = useState(false);
  const initial = member.name.trim().charAt(0).toLocaleUpperCase("pt-BR") || "?";

  return (
    <div className="mx-auto mb-2 flex aspect-[3/4] w-20 items-center justify-center overflow-hidden rounded-xl bg-base-800 ring-1 ring-base-700/60">
      {member.photo && !failed ? (
        <img
          src={member.photo}
          alt={member.name}
          loading="lazy"
          decoding="async"
          referrerPolicy="no-referrer"
          className="h-full w-full object-cover"
          onError={() => setFailed(true)}
        />
      ) : (
        <span className="text-lg font-semibold text-text-muted">{initial}</span>
      )}
    </div>
  );
}

export function CastGrid({ cast }: CastGridProps) {
  return (
    <div className="carousel-scroll flex gap-4 overflow-x-auto pb-2">
      {cast.map((member, index) => (
        <div
          key={`${member.name}-${index}`}
          className="w-24 shrink-0 snap-start text-center"
        >
          <CastAvatar member={member} />
          <p className="line-clamp-2 text-xs font-medium leading-snug text-text-secondary">
            {member.name}
          </p>
        </div>
      ))}
    </div>
  );
}
