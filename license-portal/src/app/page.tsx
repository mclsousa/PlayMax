import { Info, MonitorPlay, ShieldCheck, Zap } from "lucide-react";

import { BrandLogo } from "@/components/BrandLogo";
import { CheckoutButton } from "@/components/CheckoutButton";

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
        <h1 className="hero-title reveal reveal-2">
          Sua lista IPTV merece <em>um player à altura.</em>
        </h1>

        <p className="hero-sub reveal reveal-3">
          O Play Max é o player para Windows que organiza a lista M3U ou Xtream que você já tem —
          com capas, guia de programação e aquela cara de streaming de verdade.
        </p>

        <section className="price-card reveal reveal-4" aria-label="Assinatura">
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
        <span />
      </footer>
    </div>
  );
}
