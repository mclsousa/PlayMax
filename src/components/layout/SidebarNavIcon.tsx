import type { CSSProperties } from "react";

interface SidebarNavIconProps {
  src: string;
  active?: boolean;
  className?: string;
}

function maskStyle(src: string): CSSProperties {
  return {
    maskImage: `url(${src})`,
    WebkitMaskImage: `url(${src})`,
    maskSize: "contain",
    WebkitMaskSize: "contain",
    maskRepeat: "no-repeat",
    WebkitMaskRepeat: "no-repeat",
    maskPosition: "center",
    WebkitMaskPosition: "center",
  };
}

export function SidebarNavIcon({
  src,
  active = false,
  className = "size-5",
}: SidebarNavIconProps) {
  return (
    <span
      className={`inline-block shrink-0 transition-colors duration-250 ${
        active
          ? "bg-accent"
          : "bg-text-secondary group-hover:bg-text-primary"
      } ${className}`}
      style={maskStyle(src)}
      aria-hidden
    />
  );
}
