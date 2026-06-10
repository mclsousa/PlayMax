"use client";

import { Suspense, useEffect, useState } from "react";
import { useSearchParams } from "next/navigation";

type Phase = "loading" | "ready" | "timeout" | "invalid" | "no-session";

const MAX_ATTEMPTS = 30;
const POLL_INTERVAL_MS = 2000;

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

  const wrapper: React.CSSProperties = {
    fontFamily: "system-ui, sans-serif",
    maxWidth: 720,
    margin: "0 auto",
    padding: 32,
  };

  if (phase === "no-session") {
    return (
      <main style={wrapper}>
        <h1>Pagamento recebido</h1>
        <p>Sua licença será liberada em instantes. Você receberá a chave PLAY-XXXX por WhatsApp.</p>
      </main>
    );
  }

  if (phase === "loading") {
    return (
      <main style={wrapper}>
        <h1>Pagamento confirmado!</h1>
        <p>Gerando sua chave de licença… isso leva só alguns segundos.</p>
      </main>
    );
  }

  if (phase === "invalid") {
    return (
      <main style={wrapper}>
        <h1>Não foi possível localizar sua compra</h1>
        <p>
          Confira se você abriu o link correto após o pagamento. Se o problema continuar, fale com o
          suporte pelo WhatsApp informando o e-mail usado na compra.
        </p>
      </main>
    );
  }

  if (phase === "timeout") {
    return (
      <main style={wrapper}>
        <h1>Pagamento confirmado!</h1>
        <p>
          Sua chave está sendo gerada — atualize esta página em instantes. Se ela não aparecer, fale
          com o suporte pelo WhatsApp informando o e-mail usado na compra.
        </p>
      </main>
    );
  }

  return (
    <main style={wrapper}>
      <h1>Sua licença está pronta!</h1>
      <p>Esta é a sua chave de ativação do Play Max:</p>
      <div
        style={{
          display: "flex",
          alignItems: "center",
          gap: 12,
          background: "#ecfdf5",
          border: "1px solid #a7f3d0",
          borderRadius: 8,
          padding: 16,
          margin: "16px 0",
        }}
      >
        <code style={{ fontSize: 22, fontWeight: 700, letterSpacing: 1 }}>{licenseKey}</code>
        <button type="button" onClick={() => void copyKey()} style={{ padding: "8px 16px" }}>
          {copied ? "Copiado!" : "Copiar"}
        </button>
      </div>
      <p style={{ color: "#666" }}>Guarde esta chave — ela também fica registrada na sua compra.</p>
      <h2>Como ativar</h2>
      <ol>
        <li>Baixe e instale o Play Max no seu computador.</li>
        <li>Abra o aplicativo e cole a chave acima na tela de ativação.</li>
        <li>Adicione sua lista M3U ou Xtream e bom proveito!</li>
      </ol>
    </main>
  );
}

export default function SuccessPage() {
  return (
    <Suspense fallback={null}>
      <SuccessContent />
    </Suspense>
  );
}
