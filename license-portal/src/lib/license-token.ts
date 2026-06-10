import { createPrivateKey, sign, type KeyObject } from "node:crypto";
import type { LicenseRow } from "./supabase";

const GRACE_OFFLINE_SECS = 48 * 3600;

let cachedKey: KeyObject | null = null;

function getPrivateKey(): KeyObject {
  if (cachedKey) return cachedKey;
  const b64 = process.env.LICENSE_TOKEN_PRIVATE_KEY;
  if (!b64) {
    throw new Error("Missing LICENSE_TOKEN_PRIVATE_KEY");
  }
  cachedKey = createPrivateKey({
    key: Buffer.from(b64, "base64"),
    format: "der",
    type: "pkcs8",
  });
  return cachedKey;
}

export function signLicenseToken(license: LicenseRow, deviceFingerprint: string): string {
  const now = Math.floor(Date.now() / 1000);
  const payload = {
    v: 1,
    licenseKey: license.license_key,
    fingerprint: deviceFingerprint,
    status: license.status,
    licenseExpiresAt: license.expires_at
      ? Math.floor(new Date(license.expires_at).getTime() / 1000)
      : null,
    iat: now,
    exp: now + GRACE_OFFLINE_SECS,
  };
  const payloadBytes = Buffer.from(JSON.stringify(payload), "utf8");
  const signature = sign(null, payloadBytes, getPrivateKey());
  return `${payloadBytes.toString("base64url")}.${signature.toString("base64url")}`;
}
