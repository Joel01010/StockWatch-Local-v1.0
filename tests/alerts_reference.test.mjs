import test from 'node:test';
import assert from 'node:assert/strict';

function active(kind, threshold, value) {
  if (kind.includes('above') || kind === 'feed_stale') return value >= threshold;
  return value <= threshold;
}
function rearmReached(kind, threshold, margin, value) {
  return (kind.includes('above') || kind === 'feed_stale') ? value <= threshold - margin : value >= threshold + margin;
}
function evaluate(rule, value, nowMs) {
  if (!rule.enabled) return {trigger:false, reason:'disabled'};
  const isActive = active(rule.kind, rule.threshold, value);
  const previous = rule.lastValue;
  const lastMs = rule.lastTriggeredMs ?? -Infinity;
  const cooldownOk = nowMs - lastMs >= rule.cooldownSeconds * 1000;
  const crossed = previous !== null && !active(rule.kind, rule.threshold, previous) && isActive;
  let trigger = false;
  if (rule.repeatMode === 'once') trigger = isActive && rule.lastTriggeredMs === null;
  if (rule.repeatMode === 'crossing') trigger = isActive && rule.armed && crossed && cooldownOk;
  if (rule.repeatMode === 'repeat_while_true') trigger = isActive && cooldownOk;
  if (trigger) {
    rule.lastTriggeredMs = nowMs;
    if (rule.repeatMode === 'crossing') rule.armed = false;
    if (rule.repeatMode === 'once') rule.enabled = false;
  } else if (rule.repeatMode === 'crossing' && !rule.armed && rearmReached(rule.kind, rule.threshold, rule.resetMargin ?? 0, value)) {
    rule.armed = true;
  }
  rule.lastValue = value;
  return {trigger, active:isActive, cooldownOk};
}
function quiet(nowMinutes,start,end){if(start===end)return true;return start<end?nowMinutes>=start&&nowMinutes<end:nowMinutes>=start||nowMinutes<end}

test('crossing mode does not fire immediately on first sample already above threshold',()=>{
  const r={enabled:true,kind:'price_above',threshold:300,repeatMode:'crossing',armed:true,lastValue:null,lastTriggeredMs:null,cooldownSeconds:60,resetMargin:2};
  assert.equal(evaluate(r,301,1000).trigger,false);
  evaluate(r,299,2000);
  assert.equal(evaluate(r,301,3000).trigger,true);
});

test('hysteresis prevents ping-pong notifications until reset margin is crossed',()=>{
  const r={enabled:true,kind:'price_above',threshold:300,repeatMode:'crossing',armed:true,lastValue:299,lastTriggeredMs:null,cooldownSeconds:0,resetMargin:2};
  assert.equal(evaluate(r,301,1000).trigger,true);
  assert.equal(r.armed,false);
  evaluate(r,299,2000); assert.equal(r.armed,false);
  evaluate(r,298,3000); assert.equal(r.armed,true);
  assert.equal(evaluate(r,301,4000).trigger,true);
});

test('repeat-while-true obeys cooldown',()=>{
  const r={enabled:true,kind:'price_above',threshold:300,repeatMode:'repeat_while_true',armed:true,lastValue:299,lastTriggeredMs:null,cooldownSeconds:60,resetMargin:0};
  assert.equal(evaluate(r,301,1000).trigger,true);
  assert.equal(evaluate(r,302,30000).trigger,false);
  assert.equal(evaluate(r,303,61001).trigger,true);
});

test('once mode disables itself after the first trigger',()=>{
  const r={enabled:true,kind:'portfolio_value_above',threshold:1000,repeatMode:'once',armed:true,lastValue:null,lastTriggeredMs:null,cooldownSeconds:0,resetMargin:0};
  assert.equal(evaluate(r,1001,1000).trigger,true); assert.equal(r.enabled,false); assert.equal(evaluate(r,1100,2000).trigger,false);
});

test('quiet hours work across midnight and in daytime windows',()=>{
  assert.equal(quiet(23*60,22*60,7*60),true); assert.equal(quiet(6*60,22*60,7*60),true); assert.equal(quiet(12*60,22*60,7*60),false);
  assert.equal(quiet(13*60,12*60,14*60),true); assert.equal(quiet(15*60,12*60,14*60),false);
});

test('below thresholds rearm in the opposite direction',()=>{
  const r={enabled:true,kind:'price_below',threshold:100,repeatMode:'crossing',armed:true,lastValue:101,lastTriggeredMs:null,cooldownSeconds:0,resetMargin:2};
  assert.equal(evaluate(r,99,1000).trigger,true); evaluate(r,101,2000); assert.equal(r.armed,false); evaluate(r,102,3000); assert.equal(r.armed,true);
});

function quoteUsable(freshness) { return ['live','fresh','delayed'].includes(freshness); }
function quoteTargets({holdings=[],watchlist=[],alerts=[]}) {
  const out = new Set([...holdings, ...watchlist]);
  for (const alert of alerts) if (alert.enabled && alert.symbol) out.add(`${alert.exchange}:${alert.symbol}`);
  return [...out].sort();
}

test('stale and offline quotes are blocked from price alerts while delayed quotes remain usable',()=>{
  assert.equal(quoteUsable('live'),true); assert.equal(quoteUsable('fresh'),true); assert.equal(quoteUsable('delayed'),true);
  assert.equal(quoteUsable('stale'),false); assert.equal(quoteUsable('offline'),false);
});

test('enabled alert-only instruments become quote targets',()=>{
  const targets=quoteTargets({holdings:['NSE:DEMOTECH'],watchlist:['NSE:DEMOETF'],alerts:[{enabled:true,exchange:'NSE',symbol:'INFY'},{enabled:false,exchange:'NSE',symbol:'TCS'}]});
  assert.deepEqual(targets,['NSE:DEMOETF','NSE:DEMOTECH','NSE:INFY']);
});

test('feed-stale rules remain conceptually evaluable when a trade-price alert is freshness-blocked',()=>{
  const ageSeconds=901; const freshness='stale';
  assert.equal(quoteUsable(freshness),false);
  assert.equal(active('feed_stale',900,ageSeconds),true);
});

test('manually re-enabling a once rule can re-arm it for one more trigger',()=>{
  const r={enabled:true,kind:'price_above',threshold:300,repeatMode:'once',armed:true,lastValue:null,lastTriggeredMs:null,cooldownSeconds:0,resetMargin:0};
  assert.equal(evaluate(r,301,1000).trigger,true); assert.equal(r.enabled,false);
  // Mirrors the service policy applied when a disabled once-rule is explicitly enabled again.
  r.enabled=true; r.lastTriggeredMs=null; r.lastValue=null; r.armed=true;
  assert.equal(evaluate(r,302,2000).trigger,true); assert.equal(r.enabled,false);
});
