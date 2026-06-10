const ADULT_KEYWORDS = [
  "xxx",
  "adult",
  "adulto",
  "+18",
  "18+",
  "erotic",
  "erotico",
  "erótico",
  "porn",
  "sexy",
  "hot",
  "playboy",
  "venus",
  "onlyfans",
  "privé",
  "privado xxx",
];

export function isAdultCategory(name: string): boolean {
  const normalized = name.toLowerCase();
  return ADULT_KEYWORDS.some((keyword) => normalized.includes(keyword));
}
