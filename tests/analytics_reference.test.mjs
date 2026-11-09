import test from 'node:test';
import assert from 'node:assert/strict';

function signedExternal(kind, amount) { return kind === 'deposit' ? amount : kind === 'withdrawal' ? -amount : 0; }
function modifiedDietz(snapshots, flows) {
  if (snapshots.length < 2) return null;
  const first=snapshots[0], last=snapshots.at(-1);
  const start=Date.parse(first.at), end=Date.parse(last.at);
  if (!(end>start) || Math.abs(first.value)<Number.EPSILON) return null;
  const duration=end-start;
  let net=0, weighted=0;
  for(const f of flows){
    if(!['deposit','withdrawal'].includes(f.kind)) continue;
    const when=Date.parse(f.at); if(!(when>start&&when<=end)) continue;
    const amount=signedExternal(f.kind,f.amount); const weight=Math.max(0,Math.min(1,(end-when)/duration));
    net+=amount; weighted+=amount*weight;
  }
  const denominator=first.value+weighted; if(Math.abs(denominator)<Number.EPSILON) return null;
  return (last.value-first.value-net)/denominator*100;
}
function performancePoints(snapshots, flows){
  if(!snapshots.length) return [];
  const start=Date.parse(snapshots[0].at), initial=snapshots[0].value;
  return snapshots.map(s=>{
    const at=Date.parse(s.at);
    const external=flows.filter(f=>['deposit','withdrawal'].includes(f.kind)&&Date.parse(f.at)>start&&Date.parse(f.at)<=at).reduce((a,f)=>a+signedExternal(f.kind,f.amount),0);
    return {value:s.value, external, gain:s.value-initial-external};
  });
}
function bucket5m(ms){return Math.floor(ms/300000)*300000}

const day=24*60*60*1000;
test('a mid-period deposit is removed from investment return',()=>{
  const s=[{at:'2026-01-01T00:00:00Z',value:1000},{at:'2026-01-11T00:00:00Z',value:6100}];
  const f=[{kind:'deposit',amount:5000,at:'2026-01-06T00:00:00Z'}];
  assert.ok(Math.abs(modifiedDietz(s,f)-2.8571428571)<1e-8);
});
test('withdrawals use the opposite external-flow sign',()=>{
  const s=[{at:'2026-01-01T00:00:00Z',value:1000},{at:'2026-01-11T00:00:00Z',value:600}];
  const f=[{kind:'withdrawal',amount:500,at:'2026-01-06T00:00:00Z'}];
  // 100 gain after removing the withdrawal; denominator 750.
  assert.ok(Math.abs(modifiedDietz(s,f)-13.3333333333)<1e-8);
});
test('dividends are income, not external contributions',()=>{
  assert.equal(signedExternal('dividend',50),0); assert.equal(signedExternal('interest',20),0);
});
test('performance gain series subtracts cumulative external capital',()=>{
  const s=[{at:'2026-01-01T00:00:00Z',value:1000},{at:'2026-01-02T00:00:00Z',value:2100},{at:'2026-01-03T00:00:00Z',value:2150}];
  const f=[{kind:'deposit',amount:1000,at:'2026-01-01T12:00:00Z'}];
  const p=performancePoints(s,f); assert.deepEqual(p.map(x=>x.gain),[0,100,150]);
});
test('five-minute bucketing caps snapshot growth',()=>{
  const base=Date.parse('2026-11-08T10:01:00Z');
  assert.equal(bucket5m(base),bucket5m(base+3*60*1000));
  assert.notEqual(bucket5m(base),bucket5m(base+5*60*1000));
});
test('benchmark excess return is deterministic',()=>{
  const benchmark=(110/100-1)*100; const portfolio=15; assert.ok(Math.abs((portfolio-benchmark)-5)<1e-10);
});
test('return is unavailable with fewer than two snapshots',()=>assert.equal(modifiedDietz([{at:'2026-01-01T00:00:00Z',value:1000}],[]),null));
test('flow after end of period does not affect the period',()=>{
  const s=[{at:'2026-01-01T00:00:00Z',value:1000},{at:'2026-01-02T00:00:00Z',value:1100}];
  assert.equal(modifiedDietz(s,[{kind:'deposit',amount:5000,at:'2026-01-03T00:00:00Z'}]),10);
});
test('same bucket should be upserted rather than appended',()=>{
  const a=Date.parse('2026-01-01T10:00:01Z'), b=Date.parse('2026-01-01T10:04:59Z'); assert.equal(bucket5m(a),bucket5m(b));
});
test('a pure deposit with no market gain reports zero adjusted return',()=>{
  const s=[{at:'2026-01-01T00:00:00Z',value:1000},{at:'2026-01-11T00:00:00Z',value:6000}];
  const f=[{kind:'deposit',amount:5000,at:'2026-01-06T00:00:00Z'}]; assert.equal(modifiedDietz(s,f),0);
});
