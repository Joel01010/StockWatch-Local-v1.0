import type { DashboardSnapshot } from "./models";

export const emptySnapshot: DashboardSnapshot = {
  source: "local_database",
  marketSession: "unknown",
  providerId: null,
  lastRefreshUnixMs: null,
  refreshInFlight: false,
  lastRefreshError: null,
  activeAlertCount: 0,
  portfolio: {
    invested: 0,
    currentValue: null,
    totalPnl: null,
    totalReturnPct: null,
    dayPnl: null,
    dayReturnPct: null,
    realizedPnl: 0,
    feesPaid: 0,
    holdingsCount: 0,
  },
  holdings: [],
};
