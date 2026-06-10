import {
  Clapperboard,
  ListVideo,
  MessageCircle,
  MonitorPlay,
  Play,
  ShieldCheck,
  Sparkles,
  Tv,
  Zap,
} from "lucide-react";

import { CheckoutButton } from "@/components/CheckoutButton";

const FEATURES = [
  {
    icon: ListVideo,
    title: "Sua lista, do seu jeito",
    text: "Compatível com listas M3U e contas Xtream — cole e assista.",
  },
  {
    icon: Clapperboard,
    title: "Filmes e séries organizados",
    text: "Catálogo com capas, sinopses e continuação de onde parou.",
  },
  {
    icon: Tv,
    title: "TV ao vivo com guia",
    text: "Canais com programação completa (EPG): veja o que está passando agora e a seguir.",
  },
  {
    icon: MessageCircle,
    title: "Suporte de verdade",
    text: "Atendimento direto pelo WhatsApp sempre que precisar.",
  },
];

export default function HomePage() {
  const price = process.env.NEXT_PUBLIC_PRICE_DISPLAY;

  return (
    <div className="shell">
      <header className="nav reveal reveal-1">
        <div className="brand-mark">
          <Play size={18} strokeWidth={2.5} fill="currentColor" />
        </div>
        <span className="brand-name">Play Max</span>
      </header>

      <main className="content">
        <span className="badge reveal reveal-1">
          <Sparkles size={14} />
          Licença oficial · ativação imediata
        </span>

        <h1 className="hero-title reveal reveal-2">
          Sua TV, filmes e séries. <em>Em um só lugar.</em>
        </h1>

        <p className="hero-sub reveal reveal-3">
          O Play Max é o player IPTV para Windows que transforma a sua lista em uma experiência de
          streaming completa — rápida, bonita e sem complicação.
        </p>

        <section className="features reveal reveal-4" aria-label="Recursos">
          {FEATURES.map((feature) => (
            <article className="feature-card" key={feature.title}>
              <div className="feature-icon">
                <feature.icon size={19} />
              </div>
              <div>
                <h3>{feature.title}</h3>
                <p>{feature.text}</p>
              </div>
            </article>
          ))}
        </section>

        <section className="price-card reveal reveal-5" aria-label="Assinatura">
          <div className="price-row">
            {price ? <span className="price-value">{price}</span> : null}
            <span className="price-note">1 computador por licença · cancele quando quiser</span>
          </div>

          <CheckoutButton />

          <div className="trust-row">
            <span className="trust-item">
              <ShieldCheck size={15} />
              Pagamento seguro
            </span>
            <span className="trust-item">
              <Zap size={15} />
              Chave liberada na hora
            </span>
            <span className="trust-item">
              <MonitorPlay size={15} />
              Pronto para Windows
            </span>
          </div>
        </section>
      </main>

      <footer className="footer">
        <span>© {new Date().getFullYear()} Play Max</span>
        <a href="/admin">acesso restrito</a>
      </footer>
    </div>
  );
}
