<script lang="ts">
  import type { DashboardSnapshot } from "../lib/models";
  import { ageLabel } from "../lib/format";
  export let snapshot: DashboardSnapshot;
  export let onRefresh: () => void;
  $: provider = snapshot.providerId ? snapshot.providerId.replaceAll("_", " ").toUpperCase() : "NO PROVIDER";
</script>
<header class="topbar">
  <div><div class="eyebrow">PORTFOLIO CONTROL CENTER</div><h1>StockWatch Local</h1></div>
  <div class="top-actions">
    <div class="feed-pill" class:error={Boolean(snapshot.lastRefreshError)}><span class="feed-dot"></span>
      {snapshot.source === "offline_demo" ? "OFFLINE DEMO" : `${provider} · ${snapshot.marketSession.toUpperCase()}`} · {ageLabel(snapshot.lastRefreshUnixMs)}
    </div>
    <button class="ghost-button" on:click={onRefresh} disabled={snapshot.refreshInFlight}>{snapshot.refreshInFlight ? "Refreshing…" : "↻ Refresh"}</button>
  </div>
</header>
{#if snapshot.lastRefreshError}
  <div class="feed-error" role="status">Feed issue: {snapshot.lastRefreshError}. Cached prices remain visible with freshness labels.</div>
{/if}
