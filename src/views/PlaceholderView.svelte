<script lang="ts">
  export let title: string;
  export let eyebrow: string;
  export let phase: string;
  export let description: string;
</script>
<section class="empty-state feature-placeholder"><div class="placeholder-icon">◇</div><div class="eyebrow">{eyebrow}</div><h2>{title}</h2><p>{description}</p><span class="phase-chip">Scheduled for {phase}</span></section>
