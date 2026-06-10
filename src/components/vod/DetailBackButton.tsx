import { ChevronLeftIcon } from "../layout/icons";

interface DetailBackButtonProps {
  onClick: () => void;
}

export function DetailBackButton({ onClick }: DetailBackButtonProps) {
  return (
    <button
      type="button"
      onClick={onClick}
      className="mb-6 inline-flex items-center gap-2 rounded-full border border-white/15 bg-base-950/40 px-4 py-2 text-sm font-medium text-text-secondary backdrop-blur-sm transition-all duration-250 hover:border-white/25 hover:bg-base-800/60 hover:text-text-primary"
    >
      <ChevronLeftIcon className="h-4 w-4" />
      Voltar
    </button>
  );
}
