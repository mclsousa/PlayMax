import { test } from "node:test";
import assert from "node:assert/strict";
import { generateKeyPairSync, verify } from "node:crypto";
import type { LicenseRow } from "../src/lib/supabase";
// Import estático é seguro: o módulo só lê LICENSE_TOKEN_PRIVATE_KEY na
// primeira chamada de signLicenseToken (lazy), depois deste setup.
import { signLicenseToken } from "../src/lib/license-token";

// Gera par efêmero e injeta a privada antes de qualquer assinatura.
const { privateKey, publicKey } = generateKeyPairSync("ed25519");
process.env.LICENSE_TOKEN_PRIVATE_KEY = privateKey
  .export({ format: "der", type: "pkcs8" })
  .toString("base64");

function fakeLicense(overrides: Partial<LicenseRow> = {}): LicenseRow {
  return {
    id: "lic-1",
    customer_id: null,
    license_key: "PLAY-AAAA-BBBB-CCCC",
    status: "active",
    expires_at: "2030-01-01T00:00:00.000Z",
    stripe_subscription_id: null,
    max_devices: 1,
    notes: null,
    created_at: "2026-01-01T00:00:00.000Z",
    updated_at: "2026-01-01T00:00:00.000Z",
    ...overrides,
  };
}

test("token assinado: payload correto e assinatura verifica", () => {
  const before = Math.floor(Date.now() / 1000);
  const token = signLicenseToken(fakeLicense(), "fp-123");
  const after = Math.floor(Date.now() / 1000);

  const [payloadB64, sigB64] = token.split(".");
  const payloadBytes = Buffer.from(payloadB64, "base64url");
  const sigBytes = Buffer.from(sigB64, "base64url");

  assert.equal(verify(null, payloadBytes, publicKey, sigBytes), true);

  const claims = JSON.parse(payloadBytes.toString("utf8"));
  assert.equal(claims.v, 1);
  assert.equal(claims.licenseKey, "PLAY-AAAA-BBBB-CCCC");
  assert.equal(claims.fingerprint, "fp-123");
  assert.equal(claims.status, "active");
  assert.equal(claims.licenseExpiresAt, Math.floor(Date.parse("2030-01-01T00:00:00.000Z") / 1000));
  assert.ok(claims.iat >= before && claims.iat <= after);
  assert.equal(claims.exp, claims.iat + 48 * 3600);
});

test("licenca sem expires_at gera licenseExpiresAt null", () => {
  const token = signLicenseToken(fakeLicense({ expires_at: null }), "fp-123");
  const claims = JSON.parse(Buffer.from(token.split(".")[0], "base64url").toString("utf8"));
  assert.equal(claims.licenseExpiresAt, null);
});

test("payload adulterado nao verifica", () => {
  const token = signLicenseToken(fakeLicense(), "fp-123");
  const [payloadB64, sigB64] = token.split(".");
  const tampered = Buffer.from(payloadB64, "base64url");
  tampered[0] ^= 0xff;
  assert.equal(verify(null, tampered, publicKey, Buffer.from(sigB64, "base64url")), false);
});
