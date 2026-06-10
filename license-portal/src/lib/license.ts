import {
  generateLicenseKey,
  getSupabaseAdmin,
  isLicenseCurrentlyValid,
  licensePayload,
  normalizeLicenseKey,
  type LicenseRow,
} from "./supabase";

export class LicenseApiError extends Error {
  code: string;

  constructor(code: string, message: string) {
    super(message);
    this.code = code;
  }
}

async function fetchLicenseByKey(licenseKey: string): Promise<LicenseRow | null> {
  const supabase = getSupabaseAdmin();
  const { data, error } = await supabase
    .from("licenses")
    .select("*")
    .eq("license_key", licenseKey)
    .maybeSingle();

  if (error) {
    throw new LicenseApiError("SERVER_ERROR", error.message);
  }

  return data as LicenseRow | null;
}

async function countActivations(licenseId: string): Promise<number> {
  const supabase = getSupabaseAdmin();
  const { count, error } = await supabase
    .from("license_activations")
    .select("*", { count: "exact", head: true })
    .eq("license_id", licenseId);

  if (error) {
    throw new LicenseApiError("SERVER_ERROR", error.message);
  }

  return count ?? 0;
}

async function findActivation(licenseId: string, deviceFingerprint: string) {
  const supabase = getSupabaseAdmin();
  const { data, error } = await supabase
    .from("license_activations")
    .select("*")
    .eq("license_id", licenseId)
    .eq("device_fingerprint", deviceFingerprint)
    .maybeSingle();

  if (error) {
    throw new LicenseApiError("SERVER_ERROR", error.message);
  }

  return data;
}

export async function activateLicense(input: {
  licenseKey: string;
  deviceFingerprint: string;
  deviceName?: string | null;
}) {
  const licenseKey = normalizeLicenseKey(input.licenseKey);
  const deviceFingerprint = input.deviceFingerprint.trim();

  if (!licenseKey || !deviceFingerprint) {
    throw new LicenseApiError("INVALID_REQUEST", "Chave e dispositivo são obrigatórios.");
  }

  const license = await fetchLicenseByKey(licenseKey);
  if (!license) {
    throw new LicenseApiError("LICENSE_NOT_FOUND", "Licença não encontrada.");
  }

  if (license.status === "revoked") {
    throw new LicenseApiError("LICENSE_REVOKED", "Licença bloqueada. Fale com seu provedor.");
  }

  if (!isLicenseCurrentlyValid(license)) {
    throw new LicenseApiError(
      "LICENSE_EXPIRED",
      "Licença expirada. Renove sua assinatura para continuar.",
    );
  }

  const supabase = getSupabaseAdmin();
  const existing = await findActivation(license.id, deviceFingerprint);

  if (existing) {
    await supabase
      .from("license_activations")
      .update({ last_validated_at: new Date().toISOString(), device_name: input.deviceName ?? null })
      .eq("id", existing.id);

    return licensePayload(license);
  }

  const activationCount = await countActivations(license.id);
  if (activationCount >= license.max_devices) {
    throw new LicenseApiError(
      "DEVICE_LIMIT",
      "Esta licença já está em uso em outro computador.",
    );
  }

  const { error } = await supabase.from("license_activations").insert({
    license_id: license.id,
    device_fingerprint: deviceFingerprint,
    device_name: input.deviceName ?? null,
  });

  if (error) {
    throw new LicenseApiError("SERVER_ERROR", error.message);
  }

  return licensePayload(license);
}

export async function validateLicense(input: {
  licenseKey: string;
  deviceFingerprint: string;
}) {
  const licenseKey = normalizeLicenseKey(input.licenseKey);
  const deviceFingerprint = input.deviceFingerprint.trim();

  const license = await fetchLicenseByKey(licenseKey);
  if (!license) {
    throw new LicenseApiError("LICENSE_NOT_FOUND", "Licença não encontrada.");
  }

  if (license.status === "revoked") {
    throw new LicenseApiError("LICENSE_REVOKED", "Licença bloqueada. Fale com seu provedor.");
  }

  if (!isLicenseCurrentlyValid(license)) {
    throw new LicenseApiError(
      "LICENSE_EXPIRED",
      "Licença expirada. Renove sua assinatura para continuar.",
    );
  }

  const activation = await findActivation(license.id, deviceFingerprint);
  if (!activation) {
    throw new LicenseApiError(
      "DEVICE_NOT_ACTIVATED",
      "Este computador não está ativado. Ative a licença novamente.",
    );
  }

  const supabase = getSupabaseAdmin();
  await supabase
    .from("license_activations")
    .update({ last_validated_at: new Date().toISOString() })
    .eq("id", activation.id);

  return licensePayload(license);
}

export async function createManualLicense(input: {
  status?: LicenseRow["status"];
  expiresAt?: string | null;
  notes?: string | null;
}) {
  const supabase = getSupabaseAdmin();
  const licenseKey = generateLicenseKey();

  const { data, error } = await supabase
    .from("licenses")
    .insert({
      license_key: licenseKey,
      status: input.status ?? "active",
      expires_at: input.expiresAt ?? null,
      notes: input.notes ?? null,
      max_devices: 1,
    })
    .select("*")
    .single();

  if (error) {
    throw new LicenseApiError("SERVER_ERROR", error.message);
  }

  return data as LicenseRow;
}

export async function revokeLicense(licenseId: string) {
  const supabase = getSupabaseAdmin();
  const { error } = await supabase
    .from("licenses")
    .update({ status: "revoked" })
    .eq("id", licenseId);

  if (error) {
    throw new LicenseApiError("SERVER_ERROR", error.message);
  }
}

export async function listLicenses() {
  const supabase = getSupabaseAdmin();
  const { data, error } = await supabase
    .from("licenses")
    .select("*, license_activations(count)")
    .order("created_at", { ascending: false });

  if (error) {
    throw new LicenseApiError("SERVER_ERROR", error.message);
  }

  return data;
}

export { generateLicenseKey };
