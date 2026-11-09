<script lang="ts">
  import type { DashboardSnapshot } from "../lib/models";
  import { money, signed, signedMoney } from "../lib/format";
  export let snapshot: DashboardSnapshot;
</script>
<div class="mini-shell">
  <div class="mini-head"><div><strong>StockWatch</strong><small>{snapshot.source === "offline_demo" ? "OFFLINE" : snapshot.marketSession.toUpperCase()}</small></div><span class="mini-dot"></span></div>
  <div class="mini-holdings">
    {#each snapshot.holdings.slice(0, 6) as holding}
      <div class="mini-row"><div><strong>{holding.symbol}</strong><small>{holding.freshness.toUpperCase()}</small></div><div class="mini-price"><b>{money(holding.lastPrice)}</b><span class:positive={(holding.dayChangePct ?? 0)>=0} class:negative={(holding.dayChangePct ?? 0)<0}>{signed(holding.dayChangePct, "%")}</span></div></div>
    {/each}
  </div>
  <div class="mini-footer"><div><span>Portfolio</span><b>{money(snapshot.portfolio.currentValue)}</b></div><div><span>Today</span><b class:positive={(snapshot.portfolio.dayPnl ?? 0)>=0} class:negative={(snapshot.portfolio.dayPnl ?? 0)<0}>{signedMoney(snapshot.portfolio.dayPnl)}</b></div></div>
</div>
