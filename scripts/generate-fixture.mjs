import { writeFileSync, mkdirSync } from "node:fs";
import { join, dirname } from "node:path";
import { fileURLToPath } from "node:url";

const __dirname = dirname(fileURLToPath(import.meta.url));
const outDir = join(__dirname, "..", "tests", "fixtures");
mkdirSync(outDir, { recursive: true });

const count = Number(process.argv[2] ?? 10000);
const groups = ["Notícias", "Esportes", "Filmes", "Infantil", "Música", "Documentários"];

let content = "#EXTM3U\n";
for (let i = 1; i <= count; i += 1) {
  const group = groups[i % groups.length];
  content += `#EXTINF:-1 tvg-id="ch${i}" tvg-logo="https://via.placeholder.com/160x90?text=${i}" group-title="${group}",Canal ${i}\n`;
  content += `https://example.com/stream/${i}.m3u8\n`;
}

const outPath = join(outDir, `channels-${count}.m3u`);
writeFileSync(outPath, content, "utf8");
console.log(`Generated ${outPath} with ${count} channels.`);
