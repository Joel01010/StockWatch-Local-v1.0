import test from "node:test";
import assert from "node:assert/strict";
import fs from "node:fs";

const read = (p) => fs.readFileSync(new URL(`../${p}`, import.meta.url), "utf8");

test("architecture explicitly rejects heavyweight background stacks", () => {
  const adr = read("docs/ADR-0001-STACK.md");
  for (const forbidden of ["Electron", "Python/FastAPI", "Node background services", "Docker", "PostgreSQL", "Redis"]) {
    assert.match(adr, new RegExp(forbidden.replace("/", "\\/"), "i"));
  }
});

test("quote schema includes source and freshness", () => {
  const market = read("src-tauri/src/market/model.rs");
  assert.match(market, /pub source: String/);
  assert.match(market, /pub freshness: QuoteFreshness/);
  assert.match(market, /observed_unix_ms/);
});

test("database schema prevents negative prices and quantities", () => {
  const sql = read("src-tauri/migrations/0001_initial.sql");
  assert.match(sql, /quantity REAL NOT NULL CHECK\(quantity > 0\)/);
  assert.match(sql, /price REAL NOT NULL CHECK\(price >= 0\)/);
});
