import {
  Clapperboard,
  Info,
  ListVideo,
  MessageCircle,
  MonitorPlay,
  ShieldCheck,
  Sparkles,
  Tv,
  Zap,
} from "lucide-react";

import { BrandLogo } from "@/components/BrandLogo";
import { CheckoutButton } from "@/components/CheckoutButton";

const FEATURES = [
  {
    icon: ListVideo,
    title: "Traga a sua lista",
    text: "Cole a lista M3U ou a conta Xtream do seu provedor e comece a assistir.",
  },
  {
    icon: Clapperboard,
    title: "Catálogo organizado",
    text: "Os filmes e séries da sua lista ganham capas, sinopses e continue de onde parou.",
  },
  {
    icon: Tv,
    title: "TV ao vivo com guia",
    text: "Seus canais com guia de programação (EPG), quando o seu provedor disponibiliza.",
  },
  {
    icon: MessageCircle,
    title: "Suporte pelo WhatsApp",
    text: "Ajuda direta para ativar a licença e configurar a sua lista.",
  },
];

export default function HomePage() {
  const price = process.env.NEXT_PUBLIC_PRICE_DISPLAY;

  return (
    <div className="shell">
      <header className="nav reveal reveal-1">
        <div className="brand-mark">
          <BrandLogo size={20} />
        </div>
        <span className="brand-name">Play Max</span>
      </header>

      <main className="content">
        <span className="badge reveal reveal-1">
          <Sparkles size={14} />
          Para quem já tem lista IPTV
        </span>

        <h1 className="hero-title reveal reveal-2">
          Sua lista IPTV merece <em>um player à altura.</em>
        </h1>

        <p className="hero-sub reveal reveal-3">
          O Play Max é o player para Windows que organiza a lista M3U ou Xtream que você já tem —
          com capas, guia de programação e aquela cara de streaming de verdade.
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

        <aside className="disclaimer reveal reveal-5">
          <Info size={16} />
          <p>
            O Play Max é um aplicativo reprodutor. Não vendemos nem fornecemos canais, filmes,
            séries ou listas IPTV — para usar, você precisa da lista M3U ou Xtream do seu provedor.
          </p>
        </aside>
      </main>

      <footer className="footer">
        <span>© {new Date().getFullYear()} Play Max</span>
        <a href="/admin">acesso restrito</a>
      </footer>
    </div>
  );
}
