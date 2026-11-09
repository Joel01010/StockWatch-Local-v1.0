<script lang="ts">
  import { navigate } from "../lib/routing";
  export let current: string;
  const items = [
    ["overview", "Overview", "⌂"], ["holdings", "Holdings", "▤"], ["watchlist", "Watchlist", "☆"],
    ["alerts", "Alerts", "◉"], ["analytics", "Analytics", "⌁"], ["settings", "Settings", "⚙"],
  ];
</script>
<aside class="sidebar">
  <div class="brand"><div class="brand-mark">SW</div><div><strong>StockWatch</strong><small>LOCAL</small></div></div>
  <nav class="nav-list">
    {#each items as item}
      <button class:active={current === item[0]} on:click={() => navigate(item[0] === "overview" ? "" : item[0])}>
        <span class="nav-icon">{item[2]}</span><span>{item[1]}</span>
      </button>
    {/each}
  </nav>
  <div class="sidebar-foot"><span class="phase-dot"></span><div><b>Local-first</b><small>Tray portfolio monitor</small></div></div>
</aside>
