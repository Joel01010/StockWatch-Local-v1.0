<script lang="ts">
  import { onMount } from "svelte";
  import type { AlertCondition, AlertEventRecord, AlertRepeatMode, AlertRule, DashboardSnapshot } from "../lib/models";
  import { loadAlerts, loadAlertHistory, loadSettings, persistAlert, removeAlert } from "../lib/bridge";
  export let snapshot: DashboardSnapshot;

  let rules: AlertRule[] = [];
  let history: AlertEventRecord[] = [];
  let busy = false;
  let error = "";
  let saved = "";
  let symbol = snapshot.holdings[0]?.symbol ?? "";
  let exchange = snapshot.holdings[0]?.exchange ?? "NSE";
  let conditionKind: AlertCondition["kind"] = "price_above";
  let threshold = snapshot.holdings[0]?.lastPrice ?? 0;
  let repeatMode: AlertRepeatMode = "crossing";
  let cooldownMinutes = 15;
  let resetMargin = 1;
  let marketHoursOnly = true;

  const portfolioKinds = new Set(["portfolio_value_above", "portfolio_value_below", "portfolio_pnl_above", "portfolio_pnl_below"]);
  const percentKinds = new Set(["daily_change_above", "daily_change_below", "holding_return_above", "holding_return_below"]);

  function makeCondition(): AlertCondition {
    if (conditionKind === "feed_stale") return { kind: conditionKind, seconds: Math.max(30, Math.round(threshold)) };
    if (percentKinds.has(conditionKind)) return { kind: conditionKind as any, percent: Number(threshold) } as AlertCondition;
    return { kind: conditionKind as any, value: Number(threshold) } as AlertCondition;
  }

  async function reload() {
    rules = await loadAlerts();
    history = await loadAlertHistory(25);
  }

  async function createRule() {
    busy = true; error = ""; saved = "";
    try {
      const portfolio = portfolioKinds.has(conditionKind);
      const cleanSymbol = symbol.trim().toUpperCase();
      const cleanExchange = exchange.trim().toUpperCase();
      const rule: AlertRule = {
        id: "",
        symbol: portfolio ? null : cleanSymbol,
        exchange: portfolio ? null : cleanExchange,
        enabled: true,
        condition: makeCondition(),
        cooldown_seconds: Math.max(0, Math.round(cooldownMinutes * 60)),
        reset_margin: repeatMode === "crossing" ? Math.max(0, Number(resetMargin)) : null,
        repeat_mode: repeatMode,
        market_hours_only: marketHoursOnly,
        armed: true,
        last_value: null,
        last_triggered_at: null,
        created_at: null,
      };
      await persistAlert(rule);
      saved = "Alert saved";
      await reload();
    } catch (e) { error = String(e); }
    finally { busy = false; }
  }

  async function toggle(rule: AlertRule) { await persistAlert({ ...rule, enabled: !rule.enabled }); await reload(); }
  async function drop(id: string) { await removeAlert(id); await reload(); }
  function label(c: AlertCondition): string {
    return c.kind.replaceAll("_", " ").replace(/\b\w/g, m => m.toUpperCase()) + " " + ("value" in c ? c.value : "percent" in c ? `${c.percent}%` : `${c.seconds}s`);
  }
  function chooseHolding(value: string) {
    const found = snapshot.holdings.find(h => h.symbol === value);
    if (found) { symbol = found.symbol; exchange = found.exchange; threshold = found.lastPrice ?? threshold; }
  }

  onMount(async () => {
    await reload();
    const settings = await loadSettings();
    if (settings) cooldownMinutes = settings.notifications.default_cooldown_seconds / 60;
  });
</script>

<section class="page-title"><div class="eyebrow">RULE ENGINE</div><h2>Alerts</h2><p>Deterministic local rules with cooldowns, hysteresis, quiet hours and persistent history.</p></section>
<div class="alerts-layout">
  <section class="panel form-panel">
    <div class="panel-head"><div><h2>Create alert</h2><span class="muted">No AI · evaluated by the background Rust process</span></div></div>
    <div class="settings-form">
      <label><span>Condition</span><select bind:value={conditionKind}>
        <option value="price_above">Price above</option><option value="price_below">Price below</option>
        <option value="daily_change_above">Daily change above</option><option value="daily_change_below">Daily change below</option>
        <option value="holding_return_above">Holding return above</option><option value="holding_return_below">Holding return below</option>
        <option value="holding_pnl_above">Holding P&amp;L above</option><option value="holding_pnl_below">Holding P&amp;L below</option>
        <option value="portfolio_value_above">Portfolio value above</option><option value="portfolio_value_below">Portfolio value below</option>
        <option value="portfolio_pnl_above">Portfolio P&amp;L above</option><option value="portfolio_pnl_below">Portfolio P&amp;L below</option>
        <option value="feed_stale">Feed stale (seconds)</option>
      </select></label>

      {#if !portfolioKinds.has(conditionKind)}
        <div class="form-grid-2">
          <label><span>Symbol</span><input list="known-symbols" bind:value={symbol} on:change={(e)=>chooseHolding((e.currentTarget as HTMLInputElement).value.toUpperCase())} placeholder="RELIANCE" /></label>
          <label><span>Exchange</span><select bind:value={exchange}><option value="NSE">NSE</option><option value="BSE">BSE</option></select></label>
        </div>
        <datalist id="known-symbols">{#each snapshot.holdings as h}<option value={h.symbol}>{h.exchange} · {h.name ?? h.symbol}</option>{/each}</datalist>
        <small class="muted">You may type a custom symbol. Enabled symbol alerts are automatically included in market-data polling.</small>
      {/if}

      <label><span>{percentKinds.has(conditionKind) ? "Threshold %" : conditionKind === "feed_stale" ? "Age seconds" : "Threshold ₹"}</span><input type="number" step="0.01" bind:value={threshold} /></label>
      <label><span>Behavior</span><select bind:value={repeatMode}><option value="crossing">Crossing + rearm</option><option value="repeat_while_true">Repeat while true</option><option value="once">Once only</option></select></label>
      <div class="form-grid-2"><label><span>Cooldown minutes</span><input type="number" min="0" bind:value={cooldownMinutes}/></label><label><span>Reset margin</span><input type="number" min="0" step="0.01" bind:value={resetMargin} disabled={repeatMode !== "crossing"}/></label></div>
      <label class="toggle-row"><input type="checkbox" bind:checked={marketHoursOnly}/><span>Evaluate only during normal market hours</span></label>
      <button class="primary-button" disabled={busy || (!portfolioKinds.has(conditionKind) && !symbol.trim())} on:click={createRule}>{busy ? "Saving…" : "Create alert"}</button>
      {#if error}<div class="inline-error">{error}</div>{/if}{#if saved}<div class="inline-ok">{saved}</div>{/if}
    </div>
  </section>

  <section class="panel">
    <div class="panel-head"><div><h2>Active rules</h2><span class="muted">{rules.filter(r=>r.enabled).length} enabled · {rules.length} total</span></div></div>
    <div class="rule-list">{#if rules.length === 0}<div class="empty-row">No alerts configured yet.</div>{/if}
      {#each rules as rule}<article class="rule-row"><div><strong>{rule.symbol ?? "PORTFOLIO"}</strong><span>{label(rule.condition)}</span><small>{rule.repeat_mode.replaceAll("_"," ")} · cooldown {Math.round(rule.cooldown_seconds/60)}m{rule.last_triggered_at ? ` · last ${new Date(rule.last_triggered_at).toLocaleString()}` : ""}</small></div><div class="row-actions"><button class:enabled={rule.enabled} class="tiny-button" on:click={()=>toggle(rule)}>{rule.enabled ? "ON" : "OFF"}</button><button class="tiny-button danger" on:click={()=>drop(rule.id)}>Delete</button></div></article>{/each}
    </div>
  </section>
</div>
<section class="panel history-panel"><div class="panel-head"><div><h2>Notification history</h2><span class="muted">Stored locally; suppression and delivery failures are explicit.</span></div></div><div class="history-list">{#if history.length===0}<div class="empty-row">No alert events yet.</div>{/if}{#each history as item}<div class="history-row"><div><strong>{item.title}</strong><span>{item.body}</span></div><div><b class={item.delivery_status === "sent" ? "positive" : "muted"}>{item.delivery_status.replaceAll("_"," ")}</b><small>{new Date(item.fired_at).toLocaleString()}</small></div></div>{/each}</div></section>
