import aoVivoIcon from "../../assets/nav/ao-vivo.png";
import configIcon from "../../assets/nav/config.png";
import filmesIcon from "../../assets/nav/filmes.png";
import inicioIcon from "../../assets/nav/inicio.png";
import listaIcon from "../../assets/nav/lista.png";
import seriesIcon from "../../assets/nav/series.png";

export const sidebarNavIcons = {
  home: inicioIcon,
  live: aoVivoIcon,
  movies: filmesIcon,
  series: seriesIcon,
  list: listaIcon,
  config: configIcon,
} as const;
