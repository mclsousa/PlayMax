import { NextRequest, NextResponse } from "next/server";
import { getLicenseBySession, LicenseApiError } from "@/lib/license";

export async function GET(request: NextRequest) {
  try {
    const sessionId = request.nextUrl.searchParams.get("session_id") ?? "";
    const result = await getLicenseBySession(sessionId);
    return NextResponse.json({ ok: true, ...result });
  } catch (error) {
    if (error instanceof LicenseApiError) {
      const status =
        error.code === "INVALID_REQUEST"
          ? 400
          : error.code === "SESSION_NOT_FOUND"
            ? 404
            : error.code === "NOT_PAID"
              ? 402
              : 500;
      return NextResponse.json(
        { ok: false, code: error.code, message: error.message },
        { status },
      );
    }

    console.error("[license/by-session]", error);
    return NextResponse.json(
      { ok: false, code: "SERVER_ERROR", message: "Erro interno." },
      { status: 500 },
    );
  }
}
