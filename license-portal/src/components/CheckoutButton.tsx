"use client";

import { useState } from "react";
import { ArrowRight, Loader2 } from "lucide-react";

export function CheckoutButton() {
  const [loading, setLoading] = useState(false);

  async function startCheckout() {
    setLoading(true);
    try {
      const response = await fetch("/api/checkout", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({}),
      });

      const data = await response.json();
      if (data.url) {
        window.location.href = data.url;
        return;
      }

      alert(data.error ?? "Não foi possível iniciar o pagamento.");
      setLoading(false);
    } catch {
      alert("Não foi possível iniciar o pagamento. Tente novamente.");
      setLoading(false);
    }
  }

  return (
    <button
      type="button"
      className="cta-btn"
      onClick={() => void startCheckout()}
      disabled={loading}
    >
      {loading ? (
        <>
          <Loader2 size={19} className="spinner" />
          Abrindo pagamento seguro…
        </>
      ) : (
        <>
          Assinar Play Max
          <ArrowRight size={19} />
        </>
      )}
    </button>
  );
}
