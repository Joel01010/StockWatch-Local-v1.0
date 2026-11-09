<script lang="ts">
  import HoldingsTable from "../components/HoldingsTable.svelte";
  import type { DashboardSnapshot } from "../lib/models";
  import { money, signedMoney } from "../lib/format";
  export let snapshot: DashboardSnapshot;
</script>
<section class="page-title"><div><div class="eyebrow">PORTFOLIO</div><h2>Holdings</h2><p>Current positions derived from the local transaction ledger.</p></div></section>
<section class="metric-grid three compact">
  <div class="summary-strip"><span>Invested</span><b>{money(snapshot.portfolio.invested)}</b></div>
  <div class="summary-strip"><span>Current value</span><b>{money(snapshot.portfolio.currentValue)}</b></div>
  <div class="summary-strip"><span>Unrealized P&L</span><b class:positive={(snapshot.portfolio.totalPnl ?? 0)>=0} class:negative={(snapshot.portfolio.totalPnl ?? 0)<0}>{signedMoney(snapshot.portfolio.totalPnl)}</b></div>
</section>
<section class="panel holdings-panel"><HoldingsTable holdings={snapshot.holdings} /></section>
