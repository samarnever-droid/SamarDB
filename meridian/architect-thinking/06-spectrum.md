# 06 — SPECTRUM: the fidelity axis (and why Belady is the wrong wall)

## The assumption both documents shared

A cache may store an object byte-exact, or not store it. Belady's MIN is optimal
over exactly that feasible set. Enlarge the set and the bound is no longer a
bound — OPT was never permitted to make the new move.

## The move

A 40 KB product document: 60 % of requests need only title/price/availability/
rating (~200 B). A byte-exact cache holds the document or doesn't. A cache with a
**fidelity axis** can hold the 200 B projection and be honest about what it can
answer: in the same memory as one exact document it holds 200 projections,
satisfying 60 % of requests against 200 objects — a **120× improvement in served
requests per byte**. It is a move Belady cannot express, so measuring against
Belady cannot see it.

| Level | Representation | Serves | Bytes | Error |
|---|---|---|---|---|
| Exact | full object | everything | 40 KB | none |
| Projected | hot field subset | field queries | ~200 B | none on fields present |
| Summarised | mergeable sketch (t-digest/KLL/HLL) | aggregates, percentiles, cardinality | 1–4 KB | proven bound |
| Quantised | reduced precision / dict-coded | analytics, ranking | 10–40 % | declared ε |
| Absent | negative-cache marker | existence | 1 B | none |

## It must be type-checked, or it is data corruption

`Approx[T, ε]` **does not coerce to `T`**. A consumer that needs `Price` cannot be
served `Approx[Price, 0.01]` — the failure is a compile error at the call site.
Error bounds compose through arithmetic with interval rules, so a computation
over three 1 %-tolerant inputs carries its accumulated bound in its own type, and
a consumer demanding 1 % on that result is rejected at compile time.

The fidelity level lives in the `fid[]` plane and participates in admission: an
entry can be **demoted in fidelity instead of evicted** — strictly better than
eviction whenever any consumer tolerates the lower level.

## Honest scope

Applies only where a consumer declared tolerance (analytics, ranking, rendering,
recommendations, dashboards, search) — a minority of call sites. **Never** money,
identity, authorisation, or audit, and the type system (not a convention) is what
guarantees it. Hit ratio becomes a poorer statistic (a "hit" no longer means one
thing); the acceptance metric is served-requests-per-byte under declared
tolerance, which reduces to hit ratio when every site demands Exact.
