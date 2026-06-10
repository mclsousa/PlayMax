import { CheckoutButton } from "@/components/CheckoutButton";

export default function HomePage() {
  const price = process.env.NEXT_PUBLIC_PRICE_DISPLAY;

  return (
    <main style={{ fontFamily: "system-ui, sans-serif", maxWidth: 720, margin: "0 auto", padding: 32 }}>
      <h1>Play Max</h1>
      <p style={{ fontSize: 18 }}>
        O player IPTV para Windows: filmes, séries e TV ao vivo com a sua lista.
      </p>

      <ul style={{ lineHeight: 1.9 }}>
        <li>Funciona com listas M3U e Xtream</li>
        <li>Filmes, séries e TV ao vivo com guia de programação (EPG)</li>
        <li>1 computador por licença</li>
        <li>Suporte via WhatsApp</li>
      </ul>

      {price ? (
        <p style={{ fontSize: 24, fontWeight: 700, margin: "16px 0 8px" }}>{price}</p>
      ) : null}

      <CheckoutButton />

      <p style={{ marginTop: 16 }}>
        Após o pagamento, sua chave <strong>PLAY-XXXX-XXXX-XXXX</strong> aparece na tela na hora.
      </p>
      <p style={{ color: "#666" }}>
        Pagamento via Stripe (cartão ou Pix, conforme habilitado no Dashboard).
      </p>
      <p style={{ marginTop: 32, fontSize: 13 }}>
        <a href="/admin" style={{ color: "#999" }}>
          Painel admin
        </a>
      </p>
    </main>
  );
}
