<script lang="ts">
  import type { DashboardSnapshot } from "../lib/models";
  import HoldingsTable from "../components/HoldingsTable.svelte";
  import { ageLabel } from "../lib/format";
  export let snapshot: DashboardSnapshot;
  $: provider = snapshot.providerId?.replaceAll("_", " ") ?? "Not connected";
</script>

<section class="page-intro">
  <div><div class="eyebrow">MARKET DATA</div><h2>Tracked quotes</h2><p class="muted">Provider-backed prices with explicit source, age and freshness. Delayed data is never presented as live.</p></div>
</section>

<section class="metric-grid four">
  <div class="metric-card"><span>Provider</span><strong class="small-metric">{provider}</strong><small>Replaceable adapter</small></div>
  <div class="metric-card"><span>Session estimate</span><strong>{snapshot.marketSession.toUpperCase()}</strong><small>Normal NSE weekday clock</small></div>
  <div class="metric-card"><span>Last refresh</span><strong class="small-metric">{ageLabel(snapshot.lastRefreshUnixMs)}</strong><small>{snapshot.refreshInFlight ? "Refresh running" : "Background scheduler"}</small></div>
  <div class="metric-card"><span>Tracked holdings</span><strong>{snapshot.holdings.length}</strong><small>Holdings + DB watchlist targets</small></div>
</section>

<section class="panel holdings-panel">
  <div class="panel-head"><div><div class="eyebrow">QUOTE CACHE</div><h2>Portfolio instruments</h2></div><span class="muted">Source metadata shown per quote</span></div>
  <HoldingsTable holdings={snapshot.holdings} />
</section>

<section class="panel">
  <div class="panel-head"><div><div class="eyebrow">DATA SAFETY</div><h2>Freshness semantics</h2></div></div>
  <div class="info-grid">
    <p><strong>Delayed</strong><span>Provider declares the quote delayed; StockWatch will not relabel it as live.</span></p>
    <p><strong>Stale</strong><span>Quote exceeded the local freshness threshold but remains useful as cached context.</span></p>
    <p><strong>Offline</strong><span>Quote is too old to be treated as current. Price remains timestamped rather than disappearing.</span></p>
  </div>
</section>
