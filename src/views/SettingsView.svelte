<script lang="ts">
  import { onMount } from "svelte";
  import type { AppSettings, DashboardSnapshot, ReliabilityStatus } from "../lib/models";
  import { createBackupNow, loadReliabilityStatus, loadSettings, persistSettings, runMaintenanceNow, sendTestNotification, stageRestoreBackup } from "../lib/bridge";
  export let snapshot: DashboardSnapshot;
  let settings: AppSettings | null = null;
  let busy = false;
  let message = "";
  let error = "";
  let reliability: ReliabilityStatus | null = null;

  async function save() {
    if (!settings) return;
    busy = true; message = ""; error = "";
    try {
      const r = await persistSettings(settings);
      message = r.restartRequired ? "Saved. Restart required for provider/runtime changes." : "Saved and applied to the running background process where supported.";
    } catch (e) { error = String(e); }
    finally { busy = false; }
  }
  async function test() {
    try { await sendTestNotification(); message = "Test notification requested."; error = ""; }
    catch (e) { error = String(e); }
  }
  async function refreshReliability() { reliability = await loadReliabilityStatus(); }
  async function backupNow() {
    busy = true; message = ""; error = "";
    try { const backup = await createBackupNow(); message = `Verified backup created: ${backup.fileName}`; await refreshReliability(); }
    catch (e) { error = String(e); } finally { busy = false; }
  }
  async function maintainNow() {
    busy = true; message = ""; error = "";
    try { const report = await runMaintenanceNow(); message = `Maintenance complete. Pruned ${report.quoteHistoryRemoved + report.performanceSnapshotsRemoved + report.alertEventsRemoved} history rows.`; await refreshReliability(); }
    catch (e) { error = String(e); } finally { busy = false; }
  }
  async function restore(fileName: string) {
    if (!confirm(`Stage ${fileName} for restore on the next app restart? The current database will be preserved.`)) return;
    busy = true; message = ""; error = "";
    try { await stageRestoreBackup(fileName); message = "Restore staged and verified. Restart StockWatch Local to apply it."; await refreshReliability(); }
    catch (e) { error = String(e); } finally { busy = false; }
  }
  onMount(async () => { settings = await loadSettings(); await refreshReliability(); });
</script>

<section class="page-title"><div class="eyebrow">CONFIGURATION</div><h2>Settings</h2><p>Background behavior, market refresh, tray content and native notification policy.</p></section>
{#if !settings}
  <section class="panel"><div class="empty-row">Settings are available when running inside the Tauri application.</div></section>
{:else}
<div class="settings-stack">
  <section class="panel settings-section"><div class="panel-head"><div><h2>Background & tray</h2><span class="muted">Keep the always-on process minimal.</span></div></div><div class="settings-form settings-grid">
    <label class="toggle-row"><input type="checkbox" bind:checked={settings.launch_at_startup}/><span>Launch at Windows startup <small>applied through the native Tauri autostart plugin</small></span></label>
    <label class="toggle-row"><input type="checkbox" bind:checked={settings.tray.enabled}/><span>Enable system tray</span></label>
    <label><span>Close dashboard</span><select bind:value={settings.close_behavior}><option value="destroy_to_tray">Destroy WebView to tray (lowest RAM)</option><option value="hide_to_tray">Hide WebView (faster reopen)</option></select></label>
    <label><span>Left click tray</span><select bind:value={settings.tray.left_click_action}><option value="open_dashboard">Open dashboard</option><option value="open_mini_mode">Quick View</option><option value="none">Do nothing</option></select></label>
    <label><span>Max tray stocks</span><input type="number" min="1" max="20" bind:value={settings.tray.max_symbols}/></label>
    <label><span>Tray symbols</span><input value={settings.tray_symbols.join(", ")} on:change={(e)=>settings && (settings.tray_symbols=(e.currentTarget as HTMLInputElement).value.split(",").map(v=>v.trim()).filter(Boolean))}/><small>Comma-separated; empty uses holdings order.</small></label>
    <div class="check-grid">
      <label class="toggle-row"><input type="checkbox" bind:checked={settings.tray.show_price}/><span>Price</span></label>
      <label class="toggle-row"><input type="checkbox" bind:checked={settings.tray.show_change_pct}/><span>Daily %</span></label>
      <label class="toggle-row"><input type="checkbox" bind:checked={settings.tray.show_portfolio_value}/><span>Portfolio value</span></label>
      <label class="toggle-row"><input type="checkbox" bind:checked={settings.tray.show_day_pnl}/><span>Day P&amp;L</span></label>
      <label class="toggle-row"><input type="checkbox" bind:checked={settings.tray.show_total_pnl}/><span>Total P&amp;L</span></label>
    </div>
  </div></section>

  <section class="panel settings-section"><div class="panel-head"><div><h2>Market data</h2><span class="muted">Current provider: {snapshot.providerId ?? settings.market_data.provider}</span></div></div><div class="settings-form settings-grid">
    <label><span>Provider</span><select bind:value={settings.market_data.provider}><option value="yahoo_delayed">Yahoo delayed / best effort</option></select></label>
    <label class="toggle-row"><input type="checkbox" bind:checked={settings.market_data.refresh_on_startup}/><span>Refresh immediately at startup</span></label>
    <label><span>Open-market refresh (seconds)</span><input type="number" min="5" max="3600" bind:value={settings.refresh.open_seconds}/></label>
    <label><span>Closed-market polling</span><select value={settings.refresh.closed_seconds===null?"off":"on"} on:change={(e)=>settings&&(settings.refresh.closed_seconds=(e.currentTarget as HTMLSelectElement).value==="off"?null:(settings.refresh.closed_seconds ?? 900))}><option value="off">Stop polling</option><option value="on">Enabled</option></select></label>
    {#if settings.refresh.closed_seconds !== null}<label><span>Closed refresh (seconds)</span><input type="number" min="60" max="86400" bind:value={settings.refresh.closed_seconds}/></label>{/if}
    <label><span>Maximum failure backoff (seconds)</span><input type="number" min="5" max="3600" bind:value={settings.refresh.failure_backoff_max_seconds}/></label>
    <label><span>Request timeout (seconds)</span><input type="number" min="2" max="30" bind:value={settings.market_data.request_timeout_seconds}/></label>
    <label><span>Stale after (seconds)</span><input type="number" min="30" bind:value={settings.market_data.stale_after_seconds}/></label>
    <label><span>Offline after (seconds)</span><input type="number" min="90" bind:value={settings.market_data.offline_after_seconds}/></label>
    <label class="toggle-row"><input type="checkbox" bind:checked={settings.market_data.history_enabled}/><span>Store quote history</span></label>
  </div></section>

  <section class="panel settings-section"><div class="panel-head"><div><h2>Notifications & alert defaults</h2><span class="muted">Native delivery. Installed-build qualification is covered by the Phase 8 Windows script.</span></div><button class="ghost-button" on:click={test}>Test notification</button></div><div class="settings-form settings-grid">
    <label class="toggle-row"><input type="checkbox" bind:checked={settings.notifications.enabled}/><span>Enable notifications</span></label>
    <label class="toggle-row"><input type="checkbox" bind:checked={settings.notifications.sound}/><span>Notification sound</span></label>
    <label class="toggle-row"><input type="checkbox" bind:checked={settings.notifications.quiet_hours_enabled}/><span>Quiet hours</span></label>
    <div class="form-grid-2"><label><span>Quiet from</span><input type="time" bind:value={settings.notifications.quiet_start}/></label><label><span>Quiet until</span><input type="time" bind:value={settings.notifications.quiet_end}/></label></div>
    <label><span>Default cooldown (minutes)</span><input type="number" min="0" value={settings.notifications.default_cooldown_seconds/60} on:change={(e)=>settings&&(settings.notifications.default_cooldown_seconds=Math.max(0,Number((e.currentTarget as HTMLInputElement).value)*60))}/></label>
    <label><span>Keep notification history (days)</span><input type="number" min="1" max="365" bind:value={settings.notifications.retain_history_days}/></label>
    <div><a class="primary-button inline-link" href="#/alerts">Manage alert rules</a><small class="muted settings-note">Price, percentage, holding P&amp;L, portfolio and feed-health rules.</small></div>
  </div></section>

  <section class="panel settings-section"><div class="panel-head"><div><h2>Reliability & local data</h2><span class="muted">Verified backups, bounded history, integrity checks and crash-safe restore staging.</span></div><div class="button-row"><button class="ghost-button" disabled={busy} on:click={maintainNow}>Run maintenance</button><button class="ghost-button" disabled={busy} on:click={backupNow}>Back up now</button></div></div><div class="settings-form settings-grid">
    <label class="toggle-row"><input type="checkbox" bind:checked={settings.reliability.backup_enabled}/><span>Automatic local backups</span></label>
    <label><span>Backup interval (hours)</span><input type="number" min="1" max="168" bind:value={settings.reliability.backup_interval_hours}/></label>
    <label><span>Backups to keep</span><input type="number" min="1" max="30" bind:value={settings.reliability.backup_keep_count}/></label>
    <label><span>Quote history retention (days)</span><input type="number" min="7" max="3650" bind:value={settings.reliability.quote_history_retention_days}/></label>
    <label><span>Performance history retention (days)</span><input type="number" min="30" max="3650" bind:value={settings.reliability.performance_retention_days}/></label>
    <label><span>Maintenance interval (minutes)</span><input type="number" min="15" max="1440" bind:value={settings.reliability.maintenance_interval_minutes}/></label>
    <label><span>Event log cap (MiB)</span><input type="number" min="0.0625" max="32" step="0.25" value={settings.reliability.event_log_max_bytes/1048576} on:change={(e)=>settings&&(settings.reliability.event_log_max_bytes=Math.round(Number((e.currentTarget as HTMLInputElement).value)*1048576))}/></label>
    <label><span>Rotated log files</span><input type="number" min="1" max="10" bind:value={settings.reliability.event_log_keep_files}/></label>
  </div>
  {#if reliability}
    <div class="reliability-summary">
      <div><span>Database</span><strong class:positive={reliability.database.ok}>{reliability.database.ok ? "Healthy" : "Check failed"}</strong><small>Schema v{reliability.database.schemaVersion} · {(reliability.database.databaseBytes/1048576).toFixed(2)} MiB</small></div>
      <div><span>Refreshes</span><strong>{Number(reliability.runtime.refresh_success_count ?? 0).toLocaleString()}</strong><small>{Number(reliability.runtime.refresh_error_count ?? 0)} errors · last {reliability.runtime.last_refresh_duration_ms ?? "—"} ms</small></div>
      <div><span>Wake recovery</span><strong>{Number(reliability.runtime.wake_recovery_count ?? 0)}</strong><small>software-detected long scheduler gaps</small></div>
      <div><span>Restore state</span><strong>{reliability.pendingRestore ? "Pending restart" : "None"}</strong><small>{reliability.lastMaintenanceAt ? `maintenance ${reliability.lastMaintenanceAt}` : "restores are applied before SQLite opens"}</small></div>
    </div>
    <div class="backup-list">
      <div class="panel-subhead"><strong>Verified local backups</strong><span class="muted">{reliability.backups.length} retained</span></div>
      {#if reliability.backups.length === 0}<div class="empty-row">No backup has been created yet.</div>{/if}
      {#each reliability.backups.slice(0, 8) as backup}
        <div class="backup-row"><div><strong>{backup.fileName}</strong><small>{(backup.bytes/1048576).toFixed(2)} MiB · {backup.verified ? "verified" : "verification failed"}</small></div><button class="ghost-button" disabled={busy || !backup.verified} on:click={()=>restore(backup.fileName)}>Stage restore</button></div>
      {/each}
    </div>
  {/if}
  </section>

  <div class="settings-savebar"><div>{#if message}<span class="inline-ok">{message}</span>{/if}{#if error}<span class="inline-error">{error}</span>{/if}</div><button class="primary-button" disabled={busy} on:click={save}>{busy?"Saving…":"Save settings"}</button></div>
</div>
{/if}
