<script lang="ts">
  import { onMount } from "svelte";
  import Sidebar from "./components/Sidebar.svelte";
  import Topbar from "./components/Topbar.svelte";
  import Overview from "./views/Overview.svelte";
  import Holdings from "./views/Holdings.svelte";
  import HoldingDetail from "./views/HoldingDetail.svelte";
  import MiniMode from "./views/MiniMode.svelte";
  import MarketMonitor from "./views/MarketMonitor.svelte";
  import AlertsView from "./views/AlertsView.svelte";
  import AnalyticsView from "./views/AnalyticsView.svelte";
  import SettingsView from "./views/SettingsView.svelte";
  import { emptySnapshot } from "./lib/empty";
  import { loadDashboardSnapshot, requestRefresh } from "./lib/bridge";
  import { parseRoute, type Route } from "./lib/routing";
  import type { DashboardSnapshot } from "./lib/models";

  let route: Route = typeof window !== "undefined" ? parseRoute() : { kind: "overview" };
  let snapshot: DashboardSnapshot = emptySnapshot;
  let loading = true;

  async function reload() {
    snapshot = await loadDashboardSnapshot();
    loading = false;
  }

  async function refresh() {
    await requestRefresh();
    await reload();
  }

  onMount(() => {
    const onHash = () => route = parseRoute();
    window.addEventListener("hashchange", onHash);
    reload();
    return () => window.removeEventListener("hashchange", onHash);
  });

  $: currentNav = route.kind === "holding" ? "holdings" : route.kind;
</script>

<svelte:head><title>StockWatch Local</title><meta name="theme-color" content="#090b0e" /></svelte:head>

{#if route.kind === "mini"}
  <div class="mini-root"><MiniMode {snapshot} /></div>
{:else}
  <div class="app-shell">
    <Sidebar current={currentNav} />
    <main class="main-content">
      <Topbar {snapshot} onRefresh={refresh} />
      {#if loading}<div class="loading-line"></div>{/if}
      {#if route.kind === "overview"}
        <Overview {snapshot} />
      {:else if route.kind === "holdings"}
        <Holdings {snapshot} />
      {:else if route.kind === "holding"}
        <HoldingDetail {snapshot} symbol={route.symbol} />
      {:else if route.kind === "watchlist"}
        <MarketMonitor {snapshot} />
      {:else if route.kind === "alerts"}
        <AlertsView {snapshot} />
      {:else if route.kind === "analytics"}
        <AnalyticsView {snapshot} />
      {:else if route.kind === "settings"}
        <SettingsView {snapshot} />
      {/if}
      <footer class="app-footer">Local-first · No AI · {snapshot.providerId ? `market data: ${snapshot.providerId.replaceAll("_", " ")}` : "market data not configured"}</footer>
    </main>
  </div>
{/if}
