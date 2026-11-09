<script lang="ts">
  export let label: string;
  export let value: string;
  export let note = "";
  export let tone: "neutral" | "positive" | "negative" = "neutral";
</script>

<div class="metric-card">
  <div class="metric-label">{label}</div>
  <div class:positive={tone === "positive"} class:negative={tone === "negative"} class="metric-value">{value}</div>
  {#if note}<div class="metric-note">{note}</div>{/if}
</div>
