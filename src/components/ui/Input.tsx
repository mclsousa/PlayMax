import type { InputHTMLAttributes } from "react";

interface InputProps extends InputHTMLAttributes<HTMLInputElement> {
  label?: string;
}

export function Input({ label, className = "", id, ...props }: InputProps) {
  const inputId = id ?? label?.toLowerCase().replace(/\s+/g, "-");
  return (
    <label className="flex flex-col gap-1.5 text-sm text-text-secondary" htmlFor={inputId}>
      {label}
      <input
        id={inputId}
        className={`rounded-lg border border-base-700 bg-base-900 px-3 py-2.5 text-text-primary outline-none transition-all duration-250 placeholder:text-text-secondary/60 focus:border-accent focus:ring-2 focus:ring-accent/30 ${className}`}
        {...props}
      />
    </label>
  );
}
