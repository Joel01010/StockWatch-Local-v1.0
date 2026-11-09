<script lang="ts">
  import MetricCard from "../components/MetricCard.svelte";
  import HoldingsTable from "../components/HoldingsTable.svelte";
  import PerformanceChart from "../components/PerformanceChart.svelte";
  import type { DashboardSnapshot } from "../lib/models";
  import { demoChart } from "../lib/demo";
  import { money, signed, signedMoney } from "../lib/format";
  export let snapshot: DashboardSnapshot;
  $: p = snapshot.portfolio;
</script>

<section class="metric-grid four">
  <MetricCard label="Portfolio value" value={money(p.currentValue)} note={`Invested ${money(p.invested)}`} />
  <MetricCard label="Total P&L" value={signedMoney(p.totalPnl)} note={signed(p.totalReturnPct, "%")} tone={(p.totalPnl ?? 0) >= 0 ? "positive" : "negative"} />
  <MetricCard label="Today's P&L" value={signedMoney(p.dayPnl)} note={signed(p.dayReturnPct, "%")} tone={(p.dayPnl ?? 0) >= 0 ? "positive" : "negative"} />
  <MetricCard label="Holdings" value={String(p.holdingsCount)} note={`${snapshot.activeAlertCount} active alerts`} />
</section>

<div class="dashboard-grid">
  <section class="panel performance-panel">
    <div class="panel-head"><div><div class="eyebrow">PERFORMANCE</div><h2>Portfolio movement</h2></div><div class="segmented"><button class="active">1D</button><button>1W</button><button>1M</button><button>1Y</button><button>ALL</button></div></div>
    <PerformanceChart points={demoChart} caption={snapshot.source === "offline_demo" ? "Chart surface uses an offline demonstration series until historical snapshots arrive in Phase 6." : "Historical snapshot series will be connected in Phase 6."} />
  </section>

  <section class="panel allocation-panel">
    <div class="panel-head"><div><div class="eyebrow">ALLOCATION</div><h2>Concentration</h2></div></div>
    <div class="allocation-list">
      {#each [...snapshot.holdings].sort((a,b) => (b.allocationPct ?? 0) - (a.allocationPct ?? 0)) as holding}
        <div class="allocation-row"><div><strong>{holding.symbol}</strong><span>{holding.allocationPct?.toFixed(1) ?? "—"}%</span></div><div class="allocation-track"><i style={`width:${Math.max(0, Math.min(100, holding.allocationPct ?? 0))}%`}></i></div></div>
      {/each}
    </div>
  </section>
</div>

<section class="panel holdings-panel">
  <div class="panel-head"><div><div class="eyebrow">HOLDINGS</div><h2>Your portfolio</h2></div><span class="muted">{snapshot.source === "offline_demo" ? "Supplied screenshot · not live" : "Local SQLite portfolio"}</span></div>
  <HoldingsTable holdings={snapshot.holdings} />
</section>
