"use client";

import { Suspense, useEffect, useState } from "react";
import { useSearchParams } from "next/navigation";
import {
  Check,
  Copy,
  Download,
  KeyRound,
  LifeBuoy,
  ListVideo,
  Loader2,
  PartyPopper,
} from "lucide-react";

import { BrandLogo } from "@/components/BrandLogo";

type Phase = "loading" | "ready" | "timeout" | "invalid" | "no-session";

const MAX_ATTEMPTS = 30;
const POLL_INTERVAL_MS = 2000;

function Shell({ children }: { children: React.ReactNode }) {
  return (
    <div className="shell">
      <header className="nav reveal reveal-1">
        <div className="brand-mark">
          <BrandLogo size={20} />
        </div>
        <span className="brand-name">Play Max</span>
      </header>
      <main className="content">
        <div className="center-card">{children}</div>
      </main>
      <footer className="footer">
        <span>© {new Date().getFullYear()} Play Max</span>
        <span />
      </footer>
    </div>
  );
}

function SuccessContent() {
  const params = useSearchParams();
  const sessionId = params.get("session_id");
  const [phase, setPhase] = useState<Phase>(sessionId ? "loading" : "no-session");
  const [licenseKey, setLicenseKey] = useState<string | null>(null);
  const [copied, setCopied] = useState(false);

  useEffect(() => {
    if (!sessionId) return;
    let attempts = 0;
    let cancelled = false;
    let timer: number | undefined;

    async function poll() {
      attempts += 1;
      try {
        const response = await fetch(
          `/api/license/by-session?session_id=${encodeURIComponent(sessionId ?? "")}`,
        );
        const data = await response.json();
        if (cancelled) return;
        if (response.ok && data.ok && data.licenseKey) {
          setLicenseKey(data.licenseKey);
          setPhase("ready");
          return;
        }
        if ([400, 402, 404].includes(response.status)) {
          setPhase("invalid");
          return;
        }
      } catch {
        // falha de rede conta como tentativa; o polling continua
      }
      if (attempts >= MAX_ATTEMPTS) {
        setPhase("timeout");
        return;
      }
      timer = window.setTimeout(() => void poll(), POLL_INTERVAL_MS);
    }

    void poll();
    return () => {
      cancelled = true;
      if (timer) window.clearTimeout(timer);
    };
  }, [sessionId]);

  async function copyKey() {
    if (!licenseKey) return;
    await navigator.clipboard.writeText(licenseKey);
    setCopied(true);
    window.setTimeout(() => setCopied(false), 2000);
  }

  if (phase === "no-session") {
    return (
      <Shell>
        <div className="status-icon success reveal reveal-1">
          <PartyPopper size={30} />
        </div>
        <h1 className="success-title reveal reveal-2">Pagamento recebido!</h1>
        <p className="success-sub reveal reveal-3">
          Sua licença será liberada em instantes. Você receberá a chave de ativação pelo WhatsApp.
        </p>
      </Shell>
    );
  }

  if (phase === "loading") {
    return (
      <Shell>
        <div className="status-icon pulse reveal reveal-1">
          <Loader2 size={30} className="spinner" />
        </div>
        <h1 className="success-title reveal reveal-2">Pagamento confirmado!</h1>
        <p className="success-sub reveal reveal-3">
          Estamos gerando a sua chave de licença. Isso leva só alguns segundos…
        </p>
      </Shell>
    );
  }

  if (phase === "invalid") {
    return (
      <Shell>
        <div className="status-icon reveal reveal-1">
          <LifeBuoy size={30} />
        </div>
        <h1 className="success-title reveal reveal-2">Não localizamos sua compra</h1>
        <p className="success-sub reveal reveal-3">
          Confira se você abriu o link correto após o pagamento. Se o problema continuar, fale com o
          suporte pelo WhatsApp informando o e-mail usado na compra — resolvemos rapidinho.
        </p>
      </Shell>
    );
  }

  if (phase === "timeout") {
    return (
      <Shell>
        <div className="status-icon reveal reveal-1">
          <LifeBuoy size={30} />
        </div>
        <h1 className="success-title reveal reveal-2">Pagamento confirmado!</h1>
        <p className="success-sub reveal reveal-3">
          Sua chave está sendo gerada — atualize esta página em instantes. Se ela não aparecer, fale
          com o suporte pelo WhatsApp informando o e-mail usado na compra.
        </p>
      </Shell>
    );
  }

  return (
    <Shell>
      <div className="status-icon success reveal reveal-1">
        <PartyPopper size={30} />
      </div>
      <h1 className="success-title reveal reveal-2">Sua licença está pronta!</h1>
      <p className="success-sub reveal reveal-2">
        Esta é a sua chave de ativação. Guarde com carinho — ela também fica registrada na sua
        compra.
      </p>

      <div className="key-card reveal reveal-3">
        <span className="key-value">{licenseKey}</span>
        <button
          type="button"
          className={`copy-btn${copied ? " copied" : ""}`}
          onClick={() => void copyKey()}
        >
          {copied ? <Check size={16} /> : <Copy size={16} />}
          {copied ? "Copiado!" : "Copiar"}
        </button>
      </div>

      <div className="steps reveal reveal-4">
        <div className="step">
          <span className="step-num">1</span>
          <p>
            <strong>Baixe e instale o Play Max</strong> no seu computador Windows.
          </p>
          <Download size={18} className="step-icon" />
        </div>
        <div className="step">
          <span className="step-num">2</span>
          <p>
            <strong>Cole a chave acima</strong> na tela de ativação do aplicativo.
          </p>
          <KeyRound size={18} className="step-icon" />
        </div>
        <div className="step">
          <span className="step-num">3</span>
          <p>
            <strong>Adicione a lista</strong> M3U ou Xtream do seu provedor e aproveite!
          </p>
          <ListVideo size={18} className="step-icon" />
        </div>
      </div>
    </Shell>
  );
}

export default function SuccessPage() {
  return (
    <Suspense fallback={null}>
      <SuccessContent />
    </Suspense>
  );
}
