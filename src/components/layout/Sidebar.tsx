import { NavLink } from "react-router-dom";

import { PlayMaxSidebarLogo } from "./PlayMaxSidebarLogo";
import { SidebarNavIcon } from "./SidebarNavIcon";
import { SearchIcon } from "./icons";
import { sidebarNavIcons } from "./sidebarNavIcons";

const mainLinks = [
  { to: "/home", label: "Início", icon: sidebarNavIcons.home, iconClassName: "size-[22px]" },
  { to: "/live", label: "Ao vivo", icon: sidebarNavIcons.live },
  { to: "/movies", label: "Filmes", icon: sidebarNavIcons.movies },
  { to: "/series", label: "Séries", icon: sidebarNavIcons.series },
  { to: "/list", label: "Lista", icon: sidebarNavIcons.list },
  { to: "/search", label: "Buscar", kind: "search" as const },
];

export function Sidebar() {
  return (
    <aside className="flex w-[76px] shrink-0 flex-col items-center border-r border-base-800/60 bg-base-950 py-5">
      <div className="mb-8 flex size-11 items-center justify-center">
        <PlayMaxSidebarLogo />
      </div>

      <nav className="flex flex-1 flex-col items-center gap-2">
        {mainLinks.map((link) => {
          const { to, label } = link;
          const iconClassName = "iconClassName" in link ? link.iconClassName : undefined;

          return (
          <NavLink
            key={to}
            to={to}
            className={({ isActive }) =>
              `group flex w-full flex-col items-center gap-1 rounded-xl px-2 py-2 transition-all duration-250 ${
                isActive
                  ? "bg-accent/10 text-accent"
                  : "text-text-secondary hover:text-text-primary"
              }`
            }
          >
            {({ isActive }) => (
              <>
                {"kind" in link && link.kind === "search" ? (
                  <SearchIcon className={`size-[22px] ${isActive ? "text-accent" : ""}`} />
                ) : (
                  <SidebarNavIcon
                    src={link.icon}
                    active={isActive}
                    className={iconClassName}
                  />
                )}
                <span className={`text-[10px] font-medium ${isActive ? "text-accent" : ""}`}>
                  {label}
                </span>
              </>
            )}
          </NavLink>
          );
        })}
      </nav>

      <NavLink
        to="/settings"
        className={({ isActive }) =>
          `group mt-4 flex flex-col items-center gap-1 rounded-xl px-2 py-2 transition-all duration-250 ${
            isActive
              ? "bg-accent/10 text-accent"
              : "text-text-secondary hover:text-text-primary"
          }`
        }
      >
        {({ isActive }) => (
          <>
            <SidebarNavIcon src={sidebarNavIcons.config} active={isActive} />
            <span className={`text-[10px] font-medium ${isActive ? "text-accent" : ""}`}>
              Config
            </span>
          </>
        )}
      </NavLink>
    </aside>
  );
}
