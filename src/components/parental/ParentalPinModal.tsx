import { FormEvent, useEffect, useState } from "react";
import { Button } from "../ui/Button";
import { Input } from "../ui/Input";

interface ParentalPinModalProps {
  open: boolean;
  title: string;
  description: string;
  error?: string | null;
  submitLabel?: string;
  onClose: () => void;
  onSubmit: (pin: string) => Promise<void>;
}

export function ParentalPinModal({
  open,
  title,
  description,
  error,
  submitLabel = "Confirmar",
  onClose,
  onSubmit,
}: ParentalPinModalProps) {
  const [pin, setPin] = useState("");
  const [submitting, setSubmitting] = useState(false);

  useEffect(() => {
    if (open) {
      setPin("");
    }
  }, [open]);

  if (!open) return null;

  const handleSubmit = async (event: FormEvent) => {
    event.preventDefault();
    if (pin.trim().length !== 4) return;
    setSubmitting(true);
    try {
      await onSubmit(pin.trim());
    } finally {
      setSubmitting(false);
    }
  };

  return (
    <div className="fixed inset-0 z-[100] flex items-center justify-center bg-base-950/80 p-4 backdrop-blur-sm">
      <form
        onSubmit={handleSubmit}
        className="w-full max-w-sm rounded-2xl border border-base-700 bg-base-900 p-6 shadow-2xl"
      >
        <h2 className="mb-2 text-lg font-semibold">{title}</h2>
        <p className="mb-4 text-sm text-text-secondary">{description}</p>
        <Input
          type="password"
          inputMode="numeric"
          pattern="[0-9]*"
          maxLength={4}
          autoFocus
          value={pin}
          onChange={(event) =>
            setPin(event.target.value.replace(/\D/g, "").slice(0, 4))
          }
          placeholder="0000"
          className="mb-3 text-center text-lg tracking-[0.4em]"
        />
        {error ? <p className="mb-3 text-sm text-red-400">{error}</p> : null}
        <div className="flex justify-end gap-2">
          <Button type="button" variant="secondary" onClick={onClose}>
            Cancelar
          </Button>
          <Button type="submit" disabled={pin.length !== 4 || submitting}>
            {submitting ? "Verificando..." : submitLabel}
          </Button>
        </div>
      </form>
    </div>
  );
}
