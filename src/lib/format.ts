export const money = (value: number | null, compact = false): string => {
  if (value === null || !Number.isFinite(value)) return "—";
  return new Intl.NumberFormat("en-IN", {
    style: "currency",
    currency: "INR",
    maximumFractionDigits: compact && Math.abs(value) >= 1000 ? 0 : 2,
  }).format(value);
};

export const signed = (value: number | null, suffix = ""): string => {
  if (value === null || !Number.isFinite(value)) return "—";
  return `${value >= 0 ? "+" : ""}${value.toFixed(2)}${suffix}`;
};

export const signedMoney = (value: number | null): string => {
  if (value === null || !Number.isFinite(value)) return "—";
  return `${value >= 0 ? "+" : "−"}${money(Math.abs(value))}`;
};

export const ageLabel = (unixMs: number | null): string => {
  if (!unixMs) return "Not refreshed";
  const seconds = Math.max(0, Math.round((Date.now() - unixMs) / 1000));
  if (seconds < 60) return `${seconds}s ago`;
  const minutes = Math.floor(seconds / 60);
  if (minutes < 60) return `${minutes}m ago`;
  return `${Math.floor(minutes / 60)}h ago`;
};
