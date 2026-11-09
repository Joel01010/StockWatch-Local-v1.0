<script lang="ts">
  import { onMount } from "svelte";
  import MetricCard from "../components/MetricCard.svelte";
  import PerformanceChart from "../components/PerformanceChart.svelte";
  import { money, signed, signedMoney } from "../lib/format";
  import type { AnalyticsPreferences, AnalyticsSnapshot, CashFlowKind, CashFlowRecord, ChartPoint, DashboardSnapshot } from "../lib/models";
  import { createCashFlow, loadAnalyticsPreferences, loadAnalyticsSnapshot, loadCashFlows, persistAnalyticsPreferences, removeCashFlow } from "../lib/bridge";

  export let snapshot: DashboardSnapshot;
  let analytics: AnalyticsSnapshot | null = null;
  let cashFlows: CashFlowRecord[] = [];
  let prefs: AnalyticsPreferences = { benchmarkExchange: null, benchmarkSymbol: null };
  let loading = true;
  let message = "";
  let error = "";
  let kind: CashFlowKind = "deposit";
  let amount = "";
  let occurredAt = new Date().toISOString().slice(0,16);
  let note = "";
  let benchmarkExchange = "NSE";
  let benchmarkSymbol = "";

  $: points = (analytics?.performance ?? []).map((point) => ({
    label: new Date(point.observedAt).toLocaleDateString("en-IN", { day: "2-digit", month: "short" }), value: point.portfolioValue,
  }));

  async function reload() {
    loading = true;
    [analytics, cashFlows, prefs] = await Promise.all([loadAnalyticsSnapshot(), loadCashFlows(), loadAnalyticsPreferences()]);
    benchmarkExchange = prefs.benchmarkExchange ?? "NSE";
    benchmarkSymbol = prefs.benchmarkSymbol ?? "";
    loading = false;
  }

  async function addFlow() {
    error = ""; message = "";
    if (!amount || Number(amount) <= 0) { error = "Enter a positive amount."; return; }
    try {
      await createCashFlow({ kind, amount, occurredAt: new Date(occurredAt).toISOString(), note: note.trim() || null });
      amount = ""; note = ""; message = "Cash flow recorded locally."; await reload();
    } catch (e) { error = String(e); }
  }

  async function deleteFlow(id: number) {
    error = ""; message = "";
    try { await removeCashFlow(id); message = "Cash flow removed."; await reload(); } catch (e) { error = String(e); }
  }

  async function saveBenchmark() {
    error = ""; message = "";
    try {
      const symbol = benchmarkSymbol.trim().toUpperCase();
      const exchange = benchmarkExchange.trim().toUpperCase();
      await persistAnalyticsPreferences({ benchmarkExchange: symbol ? exchange : null, benchmarkSymbol: symbol || null });
      message = symbol ? `Benchmark set to ${exchange}:${symbol}. It will be included in future quote polling.` : "Benchmark cleared.";
      await reload();
    } catch (e) { error = String(e); }
  }

  const flowLabel = (value: CashFlowKind) => ({ deposit:"Deposit", withdrawal:"Withdrawal", dividend:"Dividend", interest:"Interest" }[value]);
  onMount(reload);
</script>

<div class="page-title analytics-title"><div><h2>Analytics</h2><p>Cash-flow-aware portfolio history, contribution analysis and recorded-charge results.</p></div><div class="tracking-chip" class:tracking={!!analytics?.snapshotCount}>{analytics?.snapshotCount ?? 0} snapshots</div></div>
{#if loading}<div class="loading-line"></div>{/if}

{#if analytics}
  <div class="metric-grid three analytics-metrics">
    <MetricCard label="Net tracked P&L" value={signedMoney(analytics.netTotalPnl)} note="Realized + unrealized + income − recorded fees" tone={(analytics.netTotalPnl ?? 0) >= 0 ? "positive" : "negative"}/>
    <MetricCard label="Cash-flow-adjusted return" value={signed(analytics.cashFlowAdjustedReturnPct, "%")} note="Modified Dietz · tracked snapshot period" tone={(analytics.cashFlowAdjustedReturnPct ?? 0) >= 0 ? "positive" : "negative"}/>
    <MetricCard label="Current securities value" value={money(analytics.currentValue)} note={`Open cost basis ${money(analytics.openCostBasis)}`}/>
  </div>

  <div class="dashboard-grid analytics-grid">
    <section class="panel"><div class="panel-head"><div><h2>Tracked performance</h2><span class="muted">External deposits/withdrawals are removed from performance math.</span></div><span class="muted">5-minute snapshot buckets</span></div>
      {#if points.length >= 2}<PerformanceChart {points} caption="Portfolio market value. Return calculation separately adjusts explicit external cash flows."/>
      {:else}<div class="analytics-empty">History begins after successful market refreshes with every open holding priced. Two snapshots are required for a return.</div>{/if}
    </section>
    <section class="panel facts-panel"><div class="panel-head"><h2>P&L composition</h2></div><dl>
      <div><dt>Unrealized</dt><dd class:positive={(analytics.unrealizedPnl??0)>=0} class:negative={(analytics.unrealizedPnl??0)<0}>{signedMoney(analytics.unrealizedPnl)}</dd></div>
      <div><dt>Realized gross</dt><dd class:positive={analytics.realizedGrossPnl>=0} class:negative={analytics.realizedGrossPnl<0}>{signedMoney(analytics.realizedGrossPnl)}</dd></div>
      <div><dt>Dividends + interest</dt><dd>{money(analytics.investmentIncome)}</dd></div>
      <div><dt>Recorded transaction fees</dt><dd class="negative">−{money(analytics.recordedFees)}</dd></div>
      <div><dt>Net external contributions</dt><dd>{money(analytics.netExternalContributions)}</dd></div>
    </dl></section>
  </div>

  <section class="panel analytics-panel"><div class="panel-head"><div><h2>Holding contribution</h2><span class="muted">Current allocation and unrealized P&L contribution.</span></div></div>
    <div class="analytics-table"><div class="analytics-row analytics-header"><span>Instrument</span><span>Value</span><span>Allocation</span><span>Unrealized P&L</span><span>P&L contribution</span></div>
    {#each analytics.contributions as row}<div class="analytics-row"><span><b>{row.symbol}</b><small>{row.exchange}</small></span><span>{money(row.currentValue)}</span><span>{row.allocationPct.toFixed(1)}%</span><span class:positive={row.unrealizedPnl>=0} class:negative={row.unrealizedPnl<0}>{signedMoney(row.unrealizedPnl)}</span><span>{row.pnlContributionPct===null?"—":signed(row.pnlContributionPct,"%")}</span></div>{/each}
    {#if !analytics.contributions.length}<div class="empty-row">No fully priced open holdings.</div>{/if}</div>
  </section>

  <div class="analytics-two-col">
    <section class="panel"><div class="panel-head"><div><h2>Transaction analytics</h2><span class="muted">Derived from the canonical local trade ledger.</span></div></div><div class="transaction-stats">
      <div><span>Buy transactions</span><b>{analytics.transactions.buyCount}</b></div><div><span>Sell transactions</span><b>{analytics.transactions.sellCount}</b></div>
      <div><span>Gross buys</span><b>{money(analytics.transactions.grossBuys)}</b></div><div><span>Gross sells</span><b>{money(analytics.transactions.grossSells)}</b></div>
      <div><span>Recorded fees</span><b>{money(analytics.transactions.recordedFees)}</b></div>
    </div></section>

    <section class="panel"><div class="panel-head"><div><h2>Benchmark</h2><span class="muted">Optional provider-polled comparison series.</span></div></div><div class="settings-form benchmark-form">
      <div class="form-grid-2"><label><span>Exchange</span><select bind:value={benchmarkExchange}><option>NSE</option><option>BSE</option></select></label><label><span>Symbol</span><input type="text" placeholder="e.g. NIFTYBEES" bind:value={benchmarkSymbol}/></label></div>
      <button class="ghost-button" on:click={saveBenchmark}>Save benchmark</button>
      {#if analytics.benchmark}<div class="benchmark-result"><b>{analytics.benchmark.exchange}:{analytics.benchmark.symbol}</b><span>{signed(analytics.benchmark.returnPct,"%")} benchmark</span><span>{signed(analytics.benchmark.portfolioExcessReturnPct,"%")} portfolio excess</span></div>{:else}<small>No comparable benchmark history yet. Saving a benchmark adds it to future quote polling; comparison begins once overlapping history exists.</small>{/if}
    </div></section>
  </div>

  <div class="analytics-two-col">
    <section class="panel"><div class="panel-head"><div><h2>Cash-flow ledger</h2><span class="muted">Deposits/withdrawals adjust returns. Dividends/interest count as investment income.</span></div></div><div class="settings-form cashflow-form">
      <div class="form-grid-2"><label><span>Type</span><select bind:value={kind}><option value="deposit">Deposit</option><option value="withdrawal">Withdrawal</option><option value="dividend">Dividend</option><option value="interest">Interest</option></select></label><label><span>Amount</span><input type="number" min="0.01" step="0.01" placeholder="0.00" bind:value={amount}/></label></div>
      <label><span>Date/time</span><input type="datetime-local" bind:value={occurredAt}/></label><label><span>Note</span><input type="text" placeholder="Optional note" bind:value={note}/></label>
      <button class="primary-button" on:click={addFlow}>Record cash flow</button>
    </div></section>
    <section class="panel"><div class="panel-head"><div><h2>Recent cash flows</h2><span class="muted">Stored only in your local SQLite database.</span></div></div><div class="cashflow-list">
      {#each [...cashFlows].reverse().slice(0,12) as flow}<div class="cashflow-row"><div><strong>{flowLabel(flow.kind)}</strong><small>{new Date(flow.occurredAt).toLocaleString("en-IN")}{flow.note?` · ${flow.note}`:""}</small></div><div><b class:negative={flow.kind==="withdrawal"}>{flow.kind==="withdrawal"?"−":"+"}{money(flow.amount)}</b><button class="tiny-button danger" on:click={()=>deleteFlow(flow.id)}>Delete</button></div></div>{/each}
      {#if !cashFlows.length}<div class="empty-row">No external cash flows or investment income recorded.</div>{/if}
    </div></section>
  </div>

  <section class="analytics-method"><strong>Methodology</strong><p>{analytics.methodology}</p><p>StockWatch does not infer deposits from stock purchases. If you want cash-flow-adjusted returns, record actual deposits and withdrawals explicitly.</p></section>
{:else}
  <section class="panel empty-state"><div><h2>Analytics backend unavailable</h2><p>The analytics UI is implemented, but this browser preview cannot reach the local Tauri/Rust backend. No performance value is invented.</p></div></section>
{/if}
{#if message}<div class="inline-ok analytics-message">{message}</div>{/if}{#if error}<div class="inline-error analytics-message">{error}</div>{/if}
