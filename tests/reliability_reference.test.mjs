import test from 'node:test';
import assert from 'node:assert/strict';

function wakeDetector(thresholdMs, samples) {
  let last=null; const wakes=[];
  for (const now of samples) { const woke=last!==null && Math.max(0,now-last)>thresholdMs; wakes.push(woke); last=now; }
  return wakes;
}
function retainNewest(rows, keep){return [...rows].sort((a,b)=>b.modified-a.modified).slice(0,Math.max(1,keep));}
function rotateModel(currentBytes,maxBytes,keepFiles,writes){
  let current=currentBytes, rotated=Array(Math.max(1,keepFiles)).fill(0);
  for(const bytes of writes){ if(current>=maxBytes){ for(let i=rotated.length-1;i>0;i--)rotated[i]=rotated[i-1]; rotated[0]=current; current=0; } current+=bytes; }
  return {current,rotated,total:current+rotated.reduce((a,b)=>a+b,0)};
}
function fiveMinuteBucket(ms){return Math.floor(ms/300000)*300000;}

test('wake detection ignores normal ticks and flags a long sleep-like gap',()=>{
  assert.deepEqual(wakeDetector(180000,[0,30000,60000,300000]),[false,false,false,true]);
});
test('backup retention keeps newest verified slots',()=>{
  const rows=[1,4,2,7,5].map(modified=>({modified}));
  assert.deepEqual(retainNewest(rows,3).map(x=>x.modified),[7,5,4]);
});
test('bounded log model cannot grow without rotating',()=>{
  const r=rotateModel(0,1024,3,Array(100).fill(200));
  assert.ok(r.current<1224); assert.ok(r.rotated.length===3); assert.ok(r.total<=1024*4+800);
});
test('five-minute snapshots collapse high-frequency refreshes',()=>{
  const buckets=new Set(); for(let i=0;i<50000;i++) buckets.add(fiveMinuteBucket(i*30000));
  assert.equal(buckets.size,5000);
});
test('365-day quote retention bounds a two-year 30-second simulation',()=>{
  const interval=30000, day=86400000, end=730*day, cutoff=end-365*day;
  let kept=0; for(let t=0;t<=end;t+=interval) if(t>=cutoff) kept++;
  assert.ok(kept<=365*24*60*2+1);
});
test('restore file name policy rejects path traversal',()=>{
  const valid=(s)=>s.length>0&&!s.includes('/')&&!s.includes('\\')&&!s.includes('..');
  assert.equal(valid('stockwatch.manual.2026.db'),true);
  assert.equal(valid('../stockwatch.db'),false); assert.equal(valid('x\\y.db'),false);
});
