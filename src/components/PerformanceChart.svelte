<script lang="ts">
  import type { ChartPoint } from "../lib/models";
  export let points: ChartPoint[] = [];
  export let caption = "Offline demonstration series";

  const width = 760;
  const height = 235;
  const padX = 12;
  const padY = 18;
  $: values = points.map((p) => p.value);
  $: min = values.length ? Math.min(...values) : 0;
  $: max = values.length ? Math.max(...values) : 1;
  $: range = Math.max(max - min, 1);
  $: coords = points.map((point, index) => {
    const x = padX + (index / Math.max(points.length - 1, 1)) * (width - padX * 2);
    const y = height - padY - ((point.value - min) / range) * (height - padY * 2);
    return { ...point, x, y };
  });
  $: path = coords.map((point, index) => `${index === 0 ? "M" : "L"}${point.x.toFixed(1)},${point.y.toFixed(1)}`).join(" ");
  $: area = coords.length ? `${path} L${coords.at(-1)?.x},${height - padY} L${coords[0].x},${height - padY} Z` : "";
</script>

<div class="chart-wrap">
  <svg viewBox={`0 0 ${width} ${height}`} role="img" aria-label="Portfolio performance chart">
    <defs>
      <linearGradient id="areaFill" x1="0" y1="0" x2="0" y2="1">
        <stop offset="0%" stop-color="currentColor" stop-opacity="0.24" />
        <stop offset="100%" stop-color="currentColor" stop-opacity="0" />
      </linearGradient>
    </defs>
    <g class="chart-grid">
      <line x1="0" y1="58" x2={width} y2="58" />
      <line x1="0" y1="117" x2={width} y2="117" />
      <line x1="0" y1="176" x2={width} y2="176" />
    </g>
    {#if area}<path d={area} class="chart-area" fill="url(#areaFill)" />{/if}
    {#if path}<path d={path} class="chart-line" />{/if}
  </svg>
  <div class="chart-axis"><span>{points[0]?.label ?? "—"}</span><span>{points[Math.floor(points.length / 2)]?.label ?? "—"}</span><span>{points.at(-1)?.label ?? "—"}</span></div>
  <div class="chart-caption">{caption}</div>
</div>
