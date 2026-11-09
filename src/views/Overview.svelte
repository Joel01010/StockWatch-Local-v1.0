<script lang="ts">
  import { onMount } from "svelte";
  import MetricCard from "../components/MetricCard.svelte";
  import HoldingsTable from "../components/HoldingsTable.svelte";
  import PerformanceChart from "../components/PerformanceChart.svelte";
  import type { ChartPoint, DashboardSnapshot } from "../lib/models";
  import { loadAnalyticsSnapshot } from "../lib/bridge";
  import { money, signed, signedMoney } from "../lib/format";
  export let snapshot: DashboardSnapshot;

  let chartPoints: ChartPoint[] = [];
  let historyLoaded = false;

  $: p = snapshot.portfolio;
  $: chartCaption = !historyLoaded
    ? "Loading verified local performance history…"
    : chartPoints.length > 0
      ? `${chartPoints.length} verified local portfolio snapshots`
      : "No verified performance history yet. StockWatch stores snapshots only after every open holding has a usable market price.";

  onMount(async () => {
    const analytics = await loadAnalyticsSnapshot();
    chartPoints = (analytics?.performance ?? []).map((point) => {
      const date = new Date(point.observedAt);
      return {
        label: Number.isNaN(date.getTime())
          ? point.observedAt
          : date.toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" }),
        value: point.portfolioValue,
      };
    });
    historyLoaded = true;
  });
</script>

<section class="metric-grid four">
  <MetricCard label="Portfolio value" value={money(p.currentValue)} note={p.holdingsCount > 0 ? `Invested ${money(p.invested)}` : "No holdings imported"} />
  <MetricCard label="Total P&L" value={signedMoney(p.totalPnl)} note={signed(p.totalReturnPct, "%")} tone={p.totalPnl === null ? "neutral" : p.totalPnl >= 0 ? "positive" : "negative"} />
  <MetricCard label="Today's P&L" value={signedMoney(p.dayPnl)} note={signed(p.dayReturnPct, "%")} tone={p.dayPnl === null ? "neutral" : p.dayPnl >= 0 ? "positive" : "negative"} />
  <MetricCard label="Holdings" value={String(p.holdingsCount)} note={`${snapshot.activeAlertCount} active alerts`} />
</section>

<div class="dashboard-grid">
  <section class="panel performance-panel">
    <div class="panel-head"><div><div class="eyebrow">PERFORMANCE</div><h2>Portfolio movement</h2></div><span class="muted">Verified local history</span></div>
    <PerformanceChart points={chartPoints} caption={chartCaption} />
  </section>

  <section class="panel allocation-panel">
    <div class="panel-head"><div><div class="eyebrow">ALLOCATION</div><h2>Concentration</h2></div></div>
    <div class="allocation-list">
      {#if snapshot.holdings.length === 0}<div class="empty-row">Allocation becomes available after holdings are imported and priced.</div>{/if}
      {#each [...snapshot.holdings].sort((a,b) => (b.allocationPct ?? 0) - (a.allocationPct ?? 0)) as holding}
        <div class="allocation-row"><div><strong>{holding.symbol}</strong><span>{holding.allocationPct?.toFixed(1) ?? "—"}%</span></div><div class="allocation-track"><i style={`width:${Math.max(0, Math.min(100, holding.allocationPct ?? 0))}%`}></i></div></div>
      {/each}
    </div>
  </section>
</div>

<section class="panel holdings-panel">
  <div class="panel-head"><div><div class="eyebrow">HOLDINGS</div><h2>Your portfolio</h2></div><span class="muted">{snapshot.holdings.length > 0 ? "Local SQLite portfolio" : "No imported portfolio"}</span></div>
  <HoldingsTable holdings={snapshot.holdings} />
</section>
