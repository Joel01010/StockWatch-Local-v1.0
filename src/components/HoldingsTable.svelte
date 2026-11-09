<script lang="ts">
  import type { HoldingSummary } from "../lib/models";
  import { ageLabel, money, signed, signedMoney } from "../lib/format";
  import FreshnessBadge from "./FreshnessBadge.svelte";
  import { navigate } from "../lib/routing";
  export let holdings: HoldingSummary[] = [];
</script>

<div class="holdings-table">
  <div class="holding-row header-row">
    <span>Instrument</span><span>Qty</span><span>Avg.</span><span>LTP</span><span>Today</span><span>Total P&amp;L</span><span>Allocation</span>
  </div>
  {#each holdings as holding}
    <button class="holding-row data-row" on:click={() => navigate(`holding/${holding.symbol}`)} aria-label={`Open ${holding.symbol}`}>
      <span class="instrument-cell"><strong>{holding.symbol}</strong><small>{holding.exchange} · {holding.name ?? "Instrument"}</small></span>
      <span>{holding.quantity}</span>
      <span>{money(holding.averagePrice)}</span>
      <span class="ltp-cell"><b>{money(holding.lastPrice)}</b><FreshnessBadge freshness={holding.freshness} /><small>{holding.quoteSource ?? "no source"} · {ageLabel(holding.quoteObservedUnixMs)}</small></span>
      <span class:positive={(holding.dayChangePct ?? 0) >= 0} class:negative={(holding.dayChangePct ?? 0) < 0}>{signed(holding.dayChangePct, "%")}</span>
      <span class:positive={(holding.pnl ?? 0) >= 0} class:negative={(holding.pnl ?? 0) < 0}><b>{signedMoney(holding.pnl)}</b><small>{signed(holding.returnPct, "%")}</small></span>
      <span>{holding.allocationPct === null ? "—" : `${holding.allocationPct.toFixed(1)}%`}</span>
    </button>
  {/each}
</div>
