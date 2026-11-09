<script lang="ts">
  import type { DashboardSnapshot } from "../lib/models";
  import FreshnessBadge from "../components/FreshnessBadge.svelte";
  import PerformanceChart from "../components/PerformanceChart.svelte";
  import { money, signed, signedMoney } from "../lib/format";
  import { navigate } from "../lib/routing";
  import { loadHoldingPriceHistory } from "../lib/bridge";
  import type { ChartPoint } from "../lib/models";
  export let snapshot: DashboardSnapshot;
  export let symbol: string;
  $: holding = snapshot.holdings.find((item) => item.symbol.toUpperCase() === symbol.toUpperCase());
  let symbolChart: ChartPoint[] = [];
  let historyKey = "";
  async function reloadHistory(exchange: string, ticker: string) {
    const key = `${exchange}:${ticker}`;
    historyKey = key; symbolChart = [];
    const rows = await loadHoldingPriceHistory(exchange, ticker, 240);
    if (historyKey !== key) return;
    symbolChart = rows.map((row) => ({ label: new Date(row.observedAt).toLocaleString("en-IN", { day: "2-digit", month: "short", hour: "2-digit", minute: "2-digit" }), value: row.price }));
  }
  $: if (holding) {
    const key = `${holding.exchange}:${holding.symbol}`;
    if (key !== historyKey) void reloadHistory(holding.exchange, holding.symbol);
  }
</script>
{#if holding}
  <section class="detail-hero">
    <button class="back-link" on:click={() => navigate("holdings")}>← Holdings</button>
    <div class="detail-heading"><div><div class="eyebrow">{holding.exchange} · HOLDING</div><h2>{holding.symbol}</h2><p>{holding.name}</p></div><div class="detail-price"><FreshnessBadge freshness={holding.freshness}/><strong>{money(holding.lastPrice)}</strong><span class:positive={(holding.dayChangePct ?? 0)>=0} class:negative={(holding.dayChangePct ?? 0)<0}>{signed(holding.dayChangePct, "%")} today</span></div></div>
  </section>
  <section class="metric-grid four compact">
    <div class="summary-strip"><span>Quantity</span><b>{holding.quantity}</b></div>
    <div class="summary-strip"><span>Average price</span><b>{money(holding.averagePrice)}</b></div>
    <div class="summary-strip"><span>Current value</span><b>{money(holding.currentValue)}</b></div>
    <div class="summary-strip"><span>Total P&L</span><b class:positive={(holding.pnl ?? 0)>=0} class:negative={(holding.pnl ?? 0)<0}>{signedMoney(holding.pnl)} · {signed(holding.returnPct, "%")}</b></div>
  </section>
  <div class="dashboard-grid detail-grid">
    <section class="panel performance-panel"><div class="panel-head"><div><div class="eyebrow">PRICE HISTORY</div><h2>{holding.symbol} movement</h2></div></div>{#if symbolChart.length >= 2}<PerformanceChart points={symbolChart} caption="Local quote history recorded by the configured market-data provider." />{:else}<div class="analytics-empty">No local price history yet. StockWatch will not draw an invented series.</div>{/if}</section>
    <section class="panel facts-panel"><div class="panel-head"><div><div class="eyebrow">POSITION</div><h2>At a glance</h2></div></div><dl><div><dt>Invested</dt><dd>{money(holding.invested)}</dd></div><div><dt>Allocation</dt><dd>{holding.allocationPct?.toFixed(2) ?? "—"}%</dd></div><div><dt>Today's P&L</dt><dd>{signedMoney(holding.dayPnl)}</dd></div><div><dt>Quote state</dt><dd>{holding.freshness.toUpperCase()}</dd></div></dl><button class="primary-button" on:click={() => navigate("alerts")}>+ Add alert</button></section>
  </div>
{:else}
  <section class="empty-state"><h2>Holding not found</h2><p>{symbol} is not present in the current local portfolio.</p><button class="primary-button" on:click={() => navigate("holdings")}>Back to holdings</button></section>
{/if}
