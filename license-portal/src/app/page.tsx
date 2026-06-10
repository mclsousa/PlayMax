import { CheckoutButton } from "@/components/CheckoutButton";

export default function HomePage() {
  return (
    <main style={{ fontFamily: "system-ui, sans-serif", maxWidth: 720, margin: "0 auto", padding: 32 }}>
      <h1>Play Max — Licenças</h1>
      <p>Painel de assinatura para o aplicativo desktop Play Max.</p>
      <p>
        Após pagar, você receberá a chave <strong>PLAY-XXXX-XXXX-XXXX</strong> pelo WhatsApp ou no
        painel admin.
      </p>
      <CheckoutButton />
      <p style={{ marginTop: 24, color: "#666" }}>
        Pagamento via Stripe (cartão ou Pix, conforme habilitado no Dashboard).
      </p>
      <p>
        <a href="/admin">Painel admin</a>
      </p>
    </main>
  );
}
