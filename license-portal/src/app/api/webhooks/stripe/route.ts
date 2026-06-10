import { NextRequest, NextResponse } from "next/server";
import Stripe from "stripe";
import { addMonths, generateLicenseKey, getSupabaseAdmin } from "@/lib/supabase";
import { getStripe } from "@/lib/stripe";

function subscriptionIdFromInvoice(invoice: Stripe.Invoice): string | null {
  const raw = invoice as Stripe.Invoice & {
    subscription?: string | Stripe.Subscription | null;
  };
  if (typeof raw.subscription === "string") return raw.subscription;
  return raw.subscription?.id ?? null;
}

function periodEndFromSubscription(subscription: Stripe.Subscription): string {
  const raw = subscription as Stripe.Subscription & { current_period_end?: number };
  if (raw.current_period_end) {
    return new Date(raw.current_period_end * 1000).toISOString();
  }
  return addMonths(new Date(), 1).toISOString();
}

export async function POST(request: NextRequest) {
  const stripe = getStripe();
  const webhookSecret = process.env.STRIPE_WEBHOOK_SECRET;

  if (!webhookSecret) {
    return NextResponse.json({ error: "Missing STRIPE_WEBHOOK_SECRET" }, { status: 500 });
  }

  const signature = request.headers.get("stripe-signature");
  if (!signature) {
    return NextResponse.json({ error: "Missing signature" }, { status: 400 });
  }

  const payload = await request.text();

  let event: Stripe.Event;
  try {
    event = stripe.webhooks.constructEvent(payload, signature, webhookSecret);
  } catch (error) {
    console.error("[stripe/webhook] signature", error);
    return NextResponse.json({ error: "Invalid signature" }, { status: 400 });
  }

  const supabase = getSupabaseAdmin();

  try {
    switch (event.type) {
      case "checkout.session.completed": {
        const session = event.data.object as Stripe.Checkout.Session;
        if (session.mode !== "subscription") break;

        const customerEmail = session.customer_details?.email ?? session.customer_email;
        const stripeCustomerId =
          typeof session.customer === "string" ? session.customer : session.customer?.id ?? null;
        const stripeSubscriptionId =
          typeof session.subscription === "string"
            ? session.subscription
            : session.subscription?.id ?? null;

        let customerId: string | null = null;
        if (stripeCustomerId) {
          const { data: existingCustomer } = await supabase
            .from("customers")
            .select("id")
            .eq("stripe_customer_id", stripeCustomerId)
            .maybeSingle();

          if (existingCustomer) {
            customerId = existingCustomer.id;
          } else {
            const { data: createdCustomer } = await supabase
              .from("customers")
              .insert({
                email: customerEmail,
                stripe_customer_id: stripeCustomerId,
              })
              .select("id")
              .single();
            customerId = createdCustomer?.id ?? null;
          }
        }

        const licenseKey = generateLicenseKey();
        const expiresAt = addMonths(new Date(), 1).toISOString();

        await supabase.from("licenses").insert({
          customer_id: customerId,
          license_key: licenseKey,
          status: "active",
          expires_at: expiresAt,
          stripe_subscription_id: stripeSubscriptionId,
          stripe_checkout_session_id: session.id,
          max_devices: 1,
        });

        break;
      }

      case "invoice.paid": {
        const invoice = event.data.object as Stripe.Invoice;
        const stripeSubscriptionId = subscriptionIdFromInvoice(invoice);

        if (!stripeSubscriptionId) break;

        const expiresAt = addMonths(new Date(), 1).toISOString();
        await supabase
          .from("licenses")
          .update({ status: "active", expires_at: expiresAt })
          .eq("stripe_subscription_id", stripeSubscriptionId);

        break;
      }

      case "customer.subscription.deleted": {
        const subscription = event.data.object as Stripe.Subscription;
        await supabase
          .from("licenses")
          .update({ status: "revoked" })
          .eq("stripe_subscription_id", subscription.id);
        break;
      }

      case "customer.subscription.updated": {
        const subscription = event.data.object as Stripe.Subscription;
        if (subscription.status === "active" || subscription.status === "trialing") {
          const expiresAt = periodEndFromSubscription(subscription);

          await supabase
            .from("licenses")
            .update({
              status: subscription.status === "trialing" ? "trial" : "active",
              expires_at: expiresAt,
            })
            .eq("stripe_subscription_id", subscription.id);
        } else if (
          subscription.status === "canceled" ||
          subscription.status === "unpaid" ||
          subscription.status === "past_due"
        ) {
          await supabase
            .from("licenses")
            .update({ status: "expired" })
            .eq("stripe_subscription_id", subscription.id);
        }
        break;
      }

      default:
        break;
    }
  } catch (error) {
    console.error("[stripe/webhook]", event.type, error);
    return NextResponse.json({ error: "Webhook handler failed" }, { status: 500 });
  }

  return NextResponse.json({ received: true });
}
