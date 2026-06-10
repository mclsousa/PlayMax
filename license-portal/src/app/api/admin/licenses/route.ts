import { NextRequest, NextResponse } from "next/server";
import {
  createManualLicense,
  deleteLicense,
  LicenseApiError,
  listLicenses,
  resetActivations,
  revokeLicense,
  unrevokeLicense,
  updateLicense,
} from "@/lib/license";

function assertAdmin(request: NextRequest) {
  const expected = process.env.ADMIN_API_KEY;
  if (!expected) {
    throw new LicenseApiError("SERVER_ERROR", "ADMIN_API_KEY not configured");
  }

  const provided = request.headers.get("x-admin-key");
  if (!provided || provided !== expected) {
    throw new LicenseApiError("UNAUTHORIZED", "Não autorizado.");
  }
}

export async function GET(request: NextRequest) {
  try {
    assertAdmin(request);
    const licenses = await listLicenses();
    return NextResponse.json({ licenses });
  } catch (error) {
    if (error instanceof LicenseApiError) {
      return NextResponse.json({ error: error.message }, { status: error.code === "UNAUTHORIZED" ? 401 : 500 });
    }
    return NextResponse.json({ error: "Erro interno" }, { status: 500 });
  }
}

export async function POST(request: NextRequest) {
  try {
    assertAdmin(request);
    const body = await request.json();
    const license = await createManualLicense({
      status: body.status,
      expiresAt: body.expiresAt ?? null,
      notes: body.notes ?? null,
    });
    return NextResponse.json({ license });
  } catch (error) {
    if (error instanceof LicenseApiError) {
      return NextResponse.json({ error: error.message }, { status: 500 });
    }
    return NextResponse.json({ error: "Erro interno" }, { status: 500 });
  }
}

export async function PATCH(request: NextRequest) {
  try {
    assertAdmin(request);
    const body = await request.json();
    const licenseId = String(body.licenseId ?? "");
    const action = String(body.action ?? "");

    if (!licenseId) {
      return NextResponse.json({ error: "licenseId required" }, { status: 400 });
    }

    if (action === "revoke") {
      await revokeLicense(licenseId);
      return NextResponse.json({ ok: true });
    }

    if (action === "unrevoke") {
      await unrevokeLicense(licenseId);
      return NextResponse.json({ ok: true });
    }

    if (action === "reset_devices") {
      await resetActivations(licenseId);
      return NextResponse.json({ ok: true });
    }

    if (action === "delete") {
      await deleteLicense(licenseId);
      return NextResponse.json({ ok: true });
    }

    if (action === "update") {
      const fields: { expiresAt?: string | null; maxDevices?: number; notes?: string | null } = {};
      if ("expiresAt" in body) {
        fields.expiresAt = body.expiresAt === null ? null : String(body.expiresAt);
      }
      if ("maxDevices" in body) {
        fields.maxDevices = Number(body.maxDevices);
      }
      if ("notes" in body) {
        fields.notes = body.notes === null ? null : String(body.notes);
      }
      await updateLicense(licenseId, fields);
      return NextResponse.json({ ok: true });
    }

    return NextResponse.json({ error: "Unknown action" }, { status: 400 });
  } catch (error) {
    if (error instanceof LicenseApiError) {
      const status =
        error.code === "UNAUTHORIZED" ? 401 : error.code === "INVALID_REQUEST" ? 400 : 500;
      return NextResponse.json({ error: error.message }, { status });
    }
    return NextResponse.json({ error: "Erro interno" }, { status: 500 });
  }
}
