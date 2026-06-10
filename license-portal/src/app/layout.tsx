import type { Metadata } from "next";
import { Sora, Instrument_Sans, JetBrains_Mono } from "next/font/google";
import "./globals.css";

const display = Sora({
  subsets: ["latin"],
  weight: ["600", "700", "800"],
  variable: "--font-display",
});

const body = Instrument_Sans({
  subsets: ["latin"],
  weight: ["400", "500", "600"],
  variable: "--font-body",
});

const mono = JetBrains_Mono({
  subsets: ["latin"],
  weight: ["700"],
  variable: "--font-mono",
});

export const metadata: Metadata = {
  title: "Play Max — Player IPTV para Windows",
  description:
    "Filmes, séries e TV ao vivo com a sua lista M3U ou Xtream. Ativação imediata após o pagamento.",
};

export default function RootLayout({ children }: { children: React.ReactNode }) {
  return (
    <html lang="pt-BR">
      <body className={`${display.variable} ${body.variable} ${mono.variable}`}>
        <div className="atmosphere" aria-hidden="true" />
        {children}
      </body>
    </html>
  );
}
