import { createClient, SupabaseClient } from "@supabase/supabase-js";

export type LicenseStatus = "pending" | "active" | "expired" | "revoked" | "trial";

export interface LicenseRow {
  id: string;
  customer_id: string | null;
  license_key: string;
  status: LicenseStatus;
  expires_at: string | null;
  stripe_subscription_id: string | null;
  max_devices: number;
  notes: string | null;
  created_at: string;
  updated_at: string;
}

export interface LicenseActivationRow {
  id: string;
  license_id: string;
  device_fingerprint: string;
  device_name: string | null;
  activated_at: string;
  last_validated_at: string;
}

let adminClient: SupabaseClient | null = null;

export function getSupabaseAdmin(): SupabaseClient {
  if (adminClient) return adminClient;

  const url = process.env.NEXT_PUBLIC_SUPABASE_URL;
  const key = process.env.SUPABASE_SERVICE_ROLE_KEY;

  if (!url || !key) {
    throw new Error("Missing NEXT_PUBLIC_SUPABASE_URL or SUPABASE_SERVICE_ROLE_KEY");
  }

  adminClient = createClient(url, key, {
    auth: { persistSession: false, autoRefreshToken: false },
  });

  return adminClient;
}

export function normalizeLicenseKey(raw: string): string {
  return raw.trim().toUpperCase().replace(/\s+/g, "");
}

export function generateLicenseKey(): string {
  const segment = () =>
    crypto.getRandomValues(new Uint8Array(2)).reduce(
      (acc, byte) => acc + byte.toString(16).padStart(2, "0"),
      "",
    ).toUpperCase();

  return `PLAY-${segment()}-${segment()}-${segment()}`;
}

export function isLicenseCurrentlyValid(license: LicenseRow): boolean {
  if (license.status !== "active" && license.status !== "trial") {
    return false;
  }
  if (!license.expires_at) {
    return true;
  }
  return new Date(license.expires_at).getTime() > Date.now();
}

export function licensePayload(license: LicenseRow) {
  return {
    licenseKey: license.license_key,
    status: license.status,
    expiresAt: license.expires_at,
    valid: isLicenseCurrentlyValid(license),
  };
}

export function addMonths(date: Date, months: number): Date {
  const next = new Date(date);
  next.setMonth(next.getMonth() + months);
  return next;
}
