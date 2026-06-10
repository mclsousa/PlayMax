import { Outlet } from "react-router-dom";

/** Shared shell class for fullscreen player CSS on live, movies and series pages. */
export function PlayerRouteLayout() {
  return (
    <div className="player-route flex min-h-0 flex-1 flex-col overflow-hidden">
      <Outlet />
    </div>
  );
}
