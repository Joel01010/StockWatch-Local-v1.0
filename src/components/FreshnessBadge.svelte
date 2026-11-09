<script lang="ts">
  import type { QuoteFreshness } from "../lib/models";
  export let freshness: QuoteFreshness;
  const labels: Record<QuoteFreshness, string> = {
    live: "LIVE", fresh: "FRESH", delayed: "DELAYED", stale: "STALE", offline: "OFFLINE",
  };
</script>
<span class="freshness-badge {freshness}">{labels[freshness]}</span>
