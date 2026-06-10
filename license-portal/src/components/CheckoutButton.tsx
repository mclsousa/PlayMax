"use client";

export function CheckoutButton() {
  async function startCheckout() {
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
  }

  return (
    <button
      type="button"
      onClick={() => void startCheckout()}
      style={{
        marginTop: 16,
        padding: "12px 20px",
        background: "#6d28d9",
        color: "#fff",
        border: "none",
        borderRadius: 8,
        cursor: "pointer",
        fontSize: 16,
      }}
    >
      Assinar Play Max
    </button>
  );
}
