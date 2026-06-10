import { NextRequest, NextResponse } from "next/server";
import { appUrl, getStripe } from "@/lib/stripe";

export async function POST(request: NextRequest) {
  try {
    const body = await request.json().catch(() => ({}));
    const email = body.email ? String(body.email) : undefined;
    const priceId = process.env.NEXT_PUBLIC_STRIPE_PRICE_ID;

    if (!priceId) {
      return NextResponse.json(
        { error: "Missing NEXT_PUBLIC_STRIPE_PRICE_ID" },
        { status: 500 },
      );
    }

    const stripe = getStripe();
    const session = await stripe.checkout.sessions.create({
      mode: "subscription",
      line_items: [{ price: priceId, quantity: 1 }],
      success_url: appUrl("/success?session_id={CHECKOUT_SESSION_ID}"),
      cancel_url: appUrl("/?canceled=1"),
      customer_email: email,
      allow_promotion_codes: true,
      billing_address_collection: "auto",
      metadata: { product: "playmax-desktop" },
    });

    return NextResponse.json({ url: session.url });
  } catch (error) {
    console.error("[checkout]", error);
    return NextResponse.json({ error: "Failed to create checkout session" }, { status: 500 });
  }
}
