import playmaxIcon from "../../assets/playmax-icon.png";

interface PlayMaxSidebarLogoProps {
  className?: string;
}

export function PlayMaxSidebarLogo({
  className = "size-10",
}: PlayMaxSidebarLogoProps) {
  return (
    <img
      src={playmaxIcon}
      alt="Play Max"
      draggable={false}
      className={`block shrink-0 object-contain ${className}`}
    />
  );
}
