# Real PostgreSQL 16.3 vs SamarDB Live Physical Benchmark Runner
param (
    [string]$pgBin = "c:\Users\khati\Documents\antigravity\epic-hubble\pgsql16\pgsql\bin",
    [int]$port = 5434,
    [string]$user = "postgres",
    [string]$db = "postgres"
)

function Run-PgSql([string]$sql) {
    & "$pgBin\psql.exe" -h 127.0.0.1 -p $port -U $user -d $db -q -t -c $sql
}

Write-Output "================================================================"
Write-Output "    REAL PostgreSQL 16.3 Physical Benchmark Execution          "
Write-Output "================================================================"

# Setup schema
Run-PgSql "DROP TABLE IF EXISTS bench_lookups; CREATE TABLE bench_lookups (id INT PRIMARY KEY, val INT);"
Run-PgSql "DROP TABLE IF EXISTS bench_agg; CREATE TABLE bench_agg (id INT, dept_id INT, salary INT);"
Run-PgSql "DROP TABLE IF EXISTS orders; CREATE TABLE orders (order_id INT, user_id INT);"
Run-PgSql "DROP TABLE IF EXISTS accounts; CREATE TABLE accounts (user_id INT, balance INT);"

# ── [1/4] Real PostgreSQL 16: 1,000 Slotted Heap Inserts ─────────────────────
Write-Output "`n[BENCHMARK 1] Real PostgreSQL 16: 1,000 Row Inserts in Transaction..."
$sw = [System.Diagnostics.Stopwatch]::StartNew()
Run-PgSql "BEGIN; INSERT INTO bench_lookups (id, val) SELECT g, g * 10 FROM generate_series(1, 1000) g; COMMIT;"
$sw.Stop()
$pg_insert_ms = $sw.Elapsed.TotalMilliseconds
Write-Output "  -> PostgreSQL 16 Insert Time: $([math]::Round($pg_insert_ms, 3)) ms ($([math]::Round(1000 / ($pg_insert_ms / 1000), 0)) rows/sec)"

# ── [2/4] Real PostgreSQL 16: 1,000 Index Point Lookups ──────────────────────
Write-Output "`n[BENCHMARK 2] Real PostgreSQL 16: 1,000 Primary Key B-Tree Lookups..."
$sw = [System.Diagnostics.Stopwatch]::StartNew()
Run-PgSql 'DO $body$ BEGIN FOR i IN 1..1000 LOOP PERFORM val FROM bench_lookups WHERE id = (i % 1000) + 1; END LOOP; END $body$;'
$sw.Stop()
$pg_lookup_ms = $sw.Elapsed.TotalMilliseconds
Write-Output "  -> PostgreSQL 16 Index Lookup Time: $([math]::Round($pg_lookup_ms, 3)) ms ($([math]::Round($pg_lookup_ms / 1000 * 1000, 3)) us/op)"

# ── [3/4] Real PostgreSQL 16: Analytical GROUP BY + 5 Aggregates (5,000 rows) ─
Write-Output "`n[BENCHMARK 3] Real PostgreSQL 16: HashAggregate (5,000 rows -> 10 groups)..."
Run-PgSql "INSERT INTO bench_agg (id, dept_id, salary) SELECT g, (g % 10) + 1, g * 100 FROM generate_series(1, 5000) g;"
$sw = [System.Diagnostics.Stopwatch]::StartNew()
for ($i = 0; $i -lt 100; $i++) {
    Run-PgSql "SELECT dept_id, COUNT(*), SUM(salary), AVG(salary), MIN(salary), MAX(salary) FROM bench_agg GROUP BY dept_id;" | Out-Null
}
$sw.Stop()
$pg_agg_avg_ms = $sw.Elapsed.TotalMilliseconds / 100
Write-Output "  -> PostgreSQL 16 HashAggregate Query Time: $([math]::Round($pg_agg_avg_ms, 3)) ms/query"

# ── [4/4] Real PostgreSQL 16: Relational Inner Hash Join (1,000 x 1,000 rows) ─
Write-Output "`n[BENCHMARK 4] Real PostgreSQL 16: Hash Join (1,000 x 1,000 rows)..."
Run-PgSql "INSERT INTO orders (order_id, user_id) SELECT g, (g % 500) + 1 FROM generate_series(1, 1000) g;"
Run-PgSql "INSERT INTO accounts (user_id, balance) SELECT g, 1000 + g FROM generate_series(1, 1000) g;"
$sw = [System.Diagnostics.Stopwatch]::StartNew()
for ($i = 0; $i -lt 50; $i++) {
    Run-PgSql "SELECT o.order_id, a.balance FROM orders o JOIN accounts a ON o.user_id = a.user_id;" | Out-Null
}
$sw.Stop()
$pg_join_avg_ms = $sw.Elapsed.TotalMilliseconds / 50
Write-Output "  -> PostgreSQL 16 Hash Join Execution Time: $([math]::Round($pg_join_avg_ms, 3)) ms/join"

Write-Output "`n================================================================"
Write-Output "    REAL POSTGRESQL 16 BENCHMARK COMPLETE                       "
Write-Output "================================================================"
