import { NextRequest, NextResponse } from "next/server";
import { activateLicense, LicenseApiError } from "@/lib/license";

export async function POST(request: NextRequest) {
  try {
    const body = await request.json();
    const result = await activateLicense({
      licenseKey: String(body.licenseKey ?? ""),
      deviceFingerprint: String(body.deviceFingerprint ?? ""),
      deviceName: body.deviceName ? String(body.deviceName) : null,
    });

    return NextResponse.json({ ok: true, ...result });
  } catch (error) {
    if (error instanceof LicenseApiError) {
      return NextResponse.json(
        { ok: false, code: error.code, message: error.message },
        { status: 400 },
      );
    }

    console.error("[license/activate]", error);
    return NextResponse.json(
      { ok: false, code: "SERVER_ERROR", message: "Erro interno." },
      { status: 500 },
    );
  }
}
