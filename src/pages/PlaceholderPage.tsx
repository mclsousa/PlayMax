import { TopBar } from "../components/layout/TopBar";

interface PlaceholderPageProps {
  title: string;
}

export function PlaceholderPage({ title }: PlaceholderPageProps) {
  return (
    <div className="flex h-full flex-col app-bg">
      <TopBar showSearch={false} />
      <div className="flex flex-1 items-center justify-center bg-base-950 text-text-secondary">
        {title} — disponível em breve.
      </div>
    </div>
  );
}
