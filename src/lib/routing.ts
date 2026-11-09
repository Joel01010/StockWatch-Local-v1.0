export type Route =
  | { kind: "overview" }
  | { kind: "holdings" }
  | { kind: "holding"; symbol: string }
  | { kind: "watchlist" }
  | { kind: "alerts" }
  | { kind: "analytics" }
  | { kind: "settings" }
  | { kind: "mini" };

export function parseRoute(hash = window.location.hash): Route {
  const raw = hash.replace(/^#\/?/, "");
  if (!raw) return { kind: "overview" };
  const [first, second] = raw.split("/");
  if (first === "holding" && second) return { kind: "holding", symbol: decodeURIComponent(second).toUpperCase() };
  if (["holdings", "watchlist", "alerts", "analytics", "settings", "mini"].includes(first)) return { kind: first as Route["kind"] } as Route;
  return { kind: "overview" };
}

export function navigate(path: string): void {
  window.location.hash = `#/${path.replace(/^\//, "")}`;
}
