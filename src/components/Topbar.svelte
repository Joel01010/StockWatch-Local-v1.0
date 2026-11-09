<script lang="ts">
  import type { DashboardSnapshot } from "../lib/models";
  import { ageLabel } from "../lib/format";
  export let snapshot: DashboardSnapshot;
  export let onRefresh: () => void;
  $: provider = snapshot.providerId ? snapshot.providerId.replaceAll("_", " ").toUpperCase() : "NO PROVIDER";
  $: feedLabel = snapshot.holdings.length === 0 && snapshot.lastRefreshUnixMs === null
    ? "NO PORTFOLIO DATA"
    : `${provider} · ${snapshot.marketSession.toUpperCase()} · ${ageLabel(snapshot.lastRefreshUnixMs)}`;
</script>
<header class="topbar">
  <div><div class="eyebrow">PORTFOLIO CONTROL CENTER</div><h1>StockWatch Local</h1></div>
  <div class="top-actions">
    <div class="feed-pill" class:error={Boolean(snapshot.lastRefreshError)}><span class="feed-dot"></span>{feedLabel}</div>
    <button class="ghost-button" on:click={onRefresh} disabled={snapshot.refreshInFlight}>{snapshot.refreshInFlight ? "Refreshing…" : "↻ Refresh"}</button>
  </div>
</header>
{#if snapshot.lastRefreshError}
  <div class="feed-error" role="status">Feed issue: {snapshot.lastRefreshError}. Cached prices remain visible only with their recorded source and freshness.</div>
{/if}
