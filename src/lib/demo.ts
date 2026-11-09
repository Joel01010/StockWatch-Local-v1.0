import type { ChartPoint, DashboardSnapshot, HoldingSummary } from "./models";

const holdingsBase: Omit<HoldingSummary, "currentValue" | "pnl" | "returnPct" | "allocationPct" | "dayPnl">[] = [
  { symbol: "DEMOETF", exchange: "NSE", name: "Demo Index ETF", quantity: 3, averagePrice: 100.00, invested: 300.00, lastPrice: 105.00, dayChangePct: 0.50, freshness: "stale", quoteSource: "synthetic_demo", quoteObservedUnixMs: null },
  { symbol: "DEMOTECH", exchange: "NSE", name: "Demo Technology", quantity: 4, averagePrice: 200.00, invested: 800.01, lastPrice: 210.00, dayChangePct: 1.20, freshness: "stale", quoteSource: "synthetic_demo", quoteObservedUnixMs: null },
  { symbol: "DEMOBANK", exchange: "NSE", name: "Demo Bank Basket", quantity: 6, averagePrice: 150.00, invested: 900.00, lastPrice: 147.00, dayChangePct: -0.70, freshness: "stale", quoteSource: "synthetic_demo", quoteObservedUnixMs: null },
  { symbol: "DEMOPHARMA", exchange: "NSE", name: "Demo Pharma Basket", quantity: 2, averagePrice: 250.00, invested: 500.00, lastPrice: 260.00, dayChangePct: 0.40, freshness: "stale", quoteSource: "synthetic_demo", quoteObservedUnixMs: null },
];

const currentTotal = holdingsBase.reduce((sum, h) => sum + (h.lastPrice ?? 0) * h.quantity, 0);
export const demoHoldings: HoldingSummary[] = holdingsBase.map((holding) => {
  const currentValue = holding.lastPrice === null ? null : holding.lastPrice * holding.quantity;
  const pnl = currentValue === null ? null : currentValue - holding.invested;
  const returnPct = pnl === null ? null : (pnl / holding.invested) * 100;
  const allocationPct = currentValue === null ? null : (currentValue / currentTotal) * 100;
  const previousPrice = holding.lastPrice && holding.dayChangePct !== null
    ? holding.lastPrice / (1 + holding.dayChangePct / 100)
    : null;
  const dayPnl = holding.lastPrice !== null && previousPrice !== null
    ? (holding.lastPrice - previousPrice) * holding.quantity
    : null;
  return { ...holding, currentValue, pnl, returnPct, allocationPct, dayPnl };
});

export const demoSnapshot: DashboardSnapshot = {
  source: "offline_demo",
  marketSession: "closed",
  providerId: null,
  lastRefreshUnixMs: null,
  refreshInFlight: false,
  lastRefreshError: null,
  activeAlertCount: 0,
  portfolio: {
    invested: 2500.01,
    currentValue: 2557.00,
    totalPnl: 56.99,
    totalReturnPct: 2.28,
    dayPnl: 7.38,
    dayReturnPct: 0.29,
    realizedPnl: 0,
    feesPaid: 0,
    holdingsCount: 4,
  },
  holdings: demoHoldings,
};

export const demoChart: ChartPoint[] = [
  { label: "09:15", value: 2522.0 }, { label: "09:45", value: 2528.5 }, { label: "10:15", value: 2525.4 },
  { label: "10:45", value: 2534.8 }, { label: "11:15", value: 2539.2 }, { label: "11:45", value: 2537.7 },
  { label: "12:15", value: 2544.6 }, { label: "12:45", value: 2541.9 }, { label: "13:15", value: 2548.4 },
  { label: "13:45", value: 2546.1 }, { label: "14:15", value: 2553.2 }, { label: "14:45", value: 2550.8 },
  { label: "15:15", value: 2557.00 },
];
