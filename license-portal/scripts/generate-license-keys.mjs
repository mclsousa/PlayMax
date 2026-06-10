// license-portal/scripts/generate-license-keys.mjs
import { generateKeyPairSync } from "node:crypto";

const { privateKey, publicKey } = generateKeyPairSync("ed25519");

const pkcs8 = privateKey.export({ format: "der", type: "pkcs8" });
// SPKI DER de Ed25519: os últimos 32 bytes são a chave pública crua.
const spki = publicKey.export({ format: "der", type: "spki" });
const raw = spki.subarray(spki.length - 32);

console.log("LICENSE_TOKEN_PRIVATE_KEY (cole no Vercel e no .env.local — NUNCA commitar):");
console.log(pkcs8.toString("base64"));
console.log("");
console.log("LICENSE_PUBLIC_KEY (cole em src-tauri/src/services/license_token.rs):");
console.log(`pub const LICENSE_PUBLIC_KEY: [u8; 32] = [${Array.from(raw).join(", ")}];`);
