import { NextRequest, NextResponse } from "next/server";
import { LicenseApiError, validateLicense } from "@/lib/license";

export async function POST(request: NextRequest) {
  try {
    const body = await request.json();
    const result = await validateLicense({
      licenseKey: String(body.licenseKey ?? ""),
      deviceFingerprint: String(body.deviceFingerprint ?? ""),
    });

    return NextResponse.json({ ok: true, ...result });
  } catch (error) {
    if (error instanceof LicenseApiError) {
      const status = error.code === "LICENSE_NOT_FOUND" ? 404 : 400;
      return NextResponse.json(
        { ok: false, code: error.code, message: error.message },
        { status },
      );
    }

    console.error("[license/validate]", error);
    return NextResponse.json(
      { ok: false, code: "SERVER_ERROR", message: "Erro interno." },
      { status: 500 },
    );
  }
}
