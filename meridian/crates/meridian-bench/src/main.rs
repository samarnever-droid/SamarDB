//! meridian-bench — phase-tagged benchmark suite.
//!
//! cargo run --release -p meridian-bench -- [--suite all|engine|server|sim]
//!                                            [--phase LABEL] [--secs N]
//!                                            [--jsonl PATH]
//!
//! Every run appends one JSON line per metric to the results file
//! (default `benchmarks/results.jsonl`), tagged with the phase label, so
//! numbers accumulate across phases and regressions are diffable. The
//! per-phase contract (which benches land when, and the gates they feed) is
//! BENCHMARKS.md at the workspace root.
//!
//! Run in release only. The engine numbers are the in-process baselines that
//! Phase 1 (SIMD probe, packed entry) must beat; the server numbers measure
//! the TCP path and are not comparable to the spec §12 in-process figures.

use std::fs::OpenOptions;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use meridian_core::hash::hash_key;
use meridian_core::{Engine, EngineOptions, SetOpts};
use meridian_sim::{run_policy, zipf_trace, Belady, Lru};

struct Args {
    suite: String,
    phase: String,
    secs: f64,
    jsonl: String,
}

fn parse_args() -> Args {
    let mut a = Args {
        suite: "all".into(),
        phase: "0-1".into(),
        secs: 2.0,
        jsonl: "benchmarks/results.jsonl".into(),
    };
    let mut it = std::env::args().skip(1);
    while let Some(f) = it.next() {
        match f.as_str() {
            "--suite" => a.suite = it.next().unwrap_or_default(),
            "--phase" => a.phase = it.next().unwrap_or_default(),
            "--secs" => a.secs = it.next().and_then(|v| v.parse().ok()).unwrap_or(a.secs),
            "--jsonl" => a.jsonl = it.next().unwrap_or_default(),
            _ => {}
        }
    }
    a
}

struct BenchResult {
    bench: String,
    metric: String,
    value: f64,
    unit: String,
}

fn record(out: &mut Vec<BenchResult>, bench: &str, metric: &str, value: f64, unit: &str) {
    println!("  {bench:<24} {metric:<14} {value:>14.1} {unit}");
    let _ = std::io::stdout().flush();
    out.push(BenchResult {
        bench: bench.into(),
        metric: metric.into(),
        value,
        unit: unit.into(),
    });
}

/// Warm up, then count completions for `secs` seconds (elapsed checked every
/// 256 ops so timing overhead stays off the measured path).
fn bench_throughput<F: FnMut()>(secs: f64, mut f: F) -> f64 {
    let warm = Duration::from_secs_f64((secs * 0.25).max(0.2));
    let t = Instant::now();
    while t.elapsed() < warm {
        f();
    }
    let t = Instant::now();
    let dur = Duration::from_secs_f64(secs);
    let mut n = 0u64;
    loop {
        for _ in 0..256 {
            f();
        }
        n += 256;
        if t.elapsed() >= dur {
            break;
        }
    }
    n as f64 / t.elapsed().as_secs_f64()
}

fn percentile_us(sorted_ns: &[u64], p: f64) -> f64 {
    let idx = ((p / 100.0) * (sorted_ns.len() - 1) as f64).round() as usize;
    sorted_ns[idx] as f64 / 1000.0
}

// ------------------------------------------------------------ engine suite

const VAL_A: [u8; 16] = [b'a'; 16];
const VAL_B: [u8; 16] = [b'b'; 16];

fn engine_suite(out: &mut Vec<BenchResult>, secs: f64) {
    println!("engine (in-process):");
    let _ = std::io::stdout().flush();

    // Paired working-set sizes mirror spec §2.1: report LLC-resident and
    // DRAM-resident index-hit rates as a pair, never one alone.
    for (name, n_keys) in [("engine/get_hit_llc", 8_000usize), ("engine/get_hit_dram", 2_000_000)] {
        let e = Engine::new(EngineOptions {
            total_entries: 1 << 22,
            ..Default::default()
        });
        let keys: Vec<Vec<u8>> = (0..n_keys as u64).map(|i| format!("bench:key:{i}").into_bytes()).collect();
        print!("  ({name}: populating {n_keys} keys…)");
        let _ = std::io::stdout().flush();
        for (i, k) in keys.iter().enumerate() {
            e.set(k, &VAL_A);
            if n_keys > 100_000 && (i + 1) % 500_000 == 0 {
                print!(" {i}");
                let _ = std::io::stdout().flush();
            }
        }
        println!(" done");
        let mut i = 0usize;
        let ops = bench_throughput(secs, || {
            let _ = std::hint::black_box(e.get(&keys[i]));
            i = (i + 1) % n_keys;
        });
        record(out, name, "throughput", ops, "ops/s");
        let mut i = 0usize;
        let ops = bench_throughput(secs, || {
            let _ = std::hint::black_box(e.get_ref(&keys[i]).as_deref());
            i = (i + 1) % n_keys;
        });
        record(out, &format!("{name}_ref"), "throughput", ops, "ops/s (zero-copy)");
    }

    // SET replacing an existing key: alloc + publish + retire path.
    {
        let e = Engine::new(EngineOptions::default());
        e.set(b"bench:hot", &VAL_A);
        let ops = bench_throughput(secs, || e.set(b"bench:hot", &VAL_B));
        record(out, "engine/set_replace", "throughput", ops, "ops/s");
    }

    // Reads of keys carrying a TTL deadline: the expire-check read path.
    {
        let e = Engine::new(EngineOptions { total_entries: 1 << 20, ..Default::default() });
        let keys: Vec<Vec<u8>> = (0..10_000u64).map(|i| format!("bench:ttl:{i}").into_bytes()).collect();
        for k in &keys {
            e.set_opts(
                k,
                &VAL_A,
                &SetOpts { ttl: Some(Duration::from_secs(600)), ..Default::default() },
            );
        }
        let mut i = 0usize;
        let n = keys.len();
        let ops = bench_throughput(secs, || {
            let _ = std::hint::black_box(e.get(&keys[i]));
            i = (i + 1) % n;
        });
        record(out, "engine/ttl_read", "throughput", ops, "ops/s");
    }

    // Contended read path: one writer flipping a key, four readers.
    // The seqlock-retry count is the number of fallback events.
    {
        let e = Arc::new(Engine::new(EngineOptions { total_entries: 1 << 20, ..Default::default() }));
        e.set(b"bench:hot", &VAL_A);
        let dur = Duration::from_secs_f64(secs);
        let writer = {
            let e = e.clone();
            std::thread::spawn(move || {
                let t = Instant::now();
                let mut n = 0u64;
                while t.elapsed() < dur {
                    let v: &[u8] = if n & 1 == 0 { &VAL_A } else { &VAL_B };
                    e.set(b"bench:hot", v);
                    n += 1;
                }
                n
            })
        };
        let mut readers = Vec::new();
        for _ in 0..4 {
            let e = e.clone();
            let dur = dur;
            readers.push(std::thread::spawn(move || {
                let t = Instant::now();
                let mut hits = 0u64;
                while t.elapsed() < dur {
                    if e.get(b"bench:hot").is_some() {
                        hits += 1;
                    }
                }
                hits
            }));
        }
        let writes = writer.join().unwrap();
        let mut reads = 0u64;
        for r in readers {
            reads += r.join().unwrap();
        }
        let total = (reads + writes) as f64 / secs;
        record(out, "engine/get_hit_4r1w", "throughput", total, "ops/s");
        record(out, "engine/get_hit_4r1w", "seqlock_retries", e.stats().retries as f64, "count");
    }

    // Idle maintenance cost: wheel ticks per second with nothing pending —
    // independent of table size (the old scan sweeper walked the table).
    {
        let e = Engine::new(EngineOptions { total_entries: 1 << 16, ..Default::default() });
        let ops = bench_throughput(secs, || e.sweep());
        record(out, "engine/sweep", "throughput", ops, "ticks/s (idle wheel)");
    }

    // Wheel expiry throughput: staggered deadlines drained by sweeping —
    // maintenance work scales with expirations, not with table size.
    {
        let e = Engine::new(EngineOptions { total_entries: 1 << 18, ..Default::default() });
        let n = 100_000usize;
        print!("  (populating {n} expiring keys…)");
        let _ = std::io::stdout().flush();
        for i in 0..n as u64 {
            let ttl = Duration::from_millis(80 + (i % 100) * 4);
            e.set_opts(
                format!("bench:exp:{i}").as_bytes(),
                &VAL_A,
                &SetOpts { ttl: Some(ttl), ..Default::default() },
            );
        }
        println!(" done");
        let t = Instant::now();
        let mut guard = 0u32;
        loop {
            e.sweep();
            if e.item_count() == 0 || guard > 5_000 {
                break;
            }
            std::thread::sleep(Duration::from_millis(2));
            guard += 1;
        }
        let s = t.elapsed().as_secs_f64();
        assert_eq!(e.item_count(), 0, "wheel failed to drain within the guard");
        record(out, "engine/wheel_expire", "throughput", n as f64 / s, "expirations/s");
    }

    // Hash cost on its own: the floor under every operation.
    {
        let ops = bench_throughput(secs, || {
            let _ = std::hint::black_box(hash_key(std::hint::black_box(b"bench:key:00042")));
        });
        record(out, "engine/hash_key", "throughput", ops, "ops/s");
    }

    // The tag matcher itself (pcmpeqb/movmskb equivalent, one word).
    {
        use meridian_core::types::match_bytes;
        let w = 0x1212_3412_1212_1212_u64;
        let ops = bench_throughput(secs, || {
            let _ = std::hint::black_box(match_bytes(std::hint::black_box(w), std::hint::black_box(0x34)));
        });
        record(out, "engine/tag_match", "throughput", ops, "matches/s");
    }

    // Epoch pin/unpin pair cost, isolated (the read path pays this per get).
    {
        let ops = bench_throughput(secs, || {
            let _g = meridian_core::epoch::Guard::new();
        });
        let ns = 1e9 / ops;
        record(out, "engine/epoch_pin", "cost_ns", ns, "ns/pin-unpair");
    }

    // L0 SPRINT hot tier (Phase 4 gate): thread-private, counter-validated.
    {
        let e = Engine::new(EngineOptions { total_entries: 1 << 16, ..Default::default() });
        e.set(b"bench:l0:hot", &VAL_A);
        let _ = e.get_l0(b"bench:l0:hot"); // warm the slot
        let ops = bench_throughput(secs, || {
            let _ = std::hint::black_box(e.with_l0(b"bench:l0:hot", |v| v.len()));
        });
        let ns = 1e9 / ops;
        record(out, "engine/l0_sprint_hit", "cost_ns", ns, "ns/op");
    }

    // Hardware calibration (lmbench-style dependent pointer chase, §2.1
    // discipline): ns per dependent load at working sets sized for each
    // level. The get_hit LLC/DRAM pair must sit above these floors —
    // measured on this box, not assumed from datasheets.
    {
        println!("  (memory latency calibration…)");
        let _ = std::io::stdout().flush();
        let mut rng: u64 = 0x9e37_79b9_7f4a_7c15;
        let mut next = || {
            rng ^= rng << 13;
            rng ^= rng >> 7;
            rng ^= rng << 17;
            rng
        };
        for (name, bytes) in [
            ("engine/mem_latency_l1_16k", 16 << 10),
            ("engine/mem_latency_l2_256k", 256 << 10),
            ("engine/mem_latency_llc_8m", 8 << 20),
            ("engine/mem_latency_dram_256m", 256 << 20),
        ] {
            let n = bytes / 8;
            // random permutation → the chase visits every element once per
            // cycle, no short loops that would go cache-hot
            let mut v: Vec<u64> = (0..n as u64).collect();
            for i in (1..n).rev() {
                let j = (next() % (i as u64 + 1)) as usize;
                v.swap(i, j);
            }
            let mut idx: usize = 0;
            for _ in 0..1000 {
                idx = v[idx] as usize;
            }
            let iters = 2_000_000usize;
            let t = Instant::now();
            for _ in 0..iters {
                idx = std::hint::black_box(v[std::hint::black_box(idx)] as usize);
            }
            let ns = t.elapsed().as_secs_f64() / iters as f64 * 1e9;
            record(out, name, "load_ns", ns, "ns/dependent-load");
        }
    }

    // Probe-cost isolation: misses over a DRAM-sized resident set — no value
    // clone, no hit path; this is the number the packed COMBO layout and
    // true SIMD probe must move.
    {
        let e = Engine::new(EngineOptions {
            total_entries: 1 << 22,
            ..Default::default()
        });
        let n = 2_000_000usize;
        print!("  (populating {n} resident keys…)");
        let _ = std::io::stdout().flush();
        for i in 0..n as u64 {
            e.set(format!("bench:key:{i}").as_bytes(), &VAL_A);
        }
        println!(" done");
        let absent: Vec<Vec<u8>> = (0..1_000_000u64).map(|i| format!("bench:absent:{i}").into_bytes()).collect();
        let mut i = 0usize;
        let m = absent.len();
        let ops = bench_throughput(secs, || {
            let _ = std::hint::black_box(e.get(&absent[i]));
            i = (i + 1) % m;
        });
        record(out, "engine/lookup_miss_dram", "throughput", ops, "ops/s");
    }
}

// ------------------------------------------------------------ server suite

fn server_suite(out: &mut Vec<BenchResult>, secs: f64) {
    println!("server (TCP loopback, 1 connection):");
    let _ = std::io::stdout().flush();
    let engine = Arc::new(Engine::new(EngineOptions::default()));
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    std::thread::spawn(move || meridian_server::serve(engine, listener).unwrap());

    let mut s = TcpStream::connect(addr).unwrap();
    s.set_nodelay(true).unwrap();
    // A framing or accounting bug must surface as an error, never a hang.
    let guard = Duration::from_secs(10);
    s.set_read_timeout(Some(guard)).unwrap();
    s.set_write_timeout(Some(guard)).unwrap();

    // Sequential PING RTT distribution: the tail is what §9 cares about.
    {
        let mut samples = Vec::with_capacity(400_000);
        let dur = Duration::from_secs_f64(secs);
        let t = Instant::now();
        while t.elapsed() < dur {
            let t0 = Instant::now();
            s.write_all(b"*1\r\n$4\r\nPING\r\n").unwrap();
            let mut buf = [0u8; 6];
            s.read_exact(&mut buf).unwrap();
            samples.push(t0.elapsed().as_nanos() as u64);
        }
        samples.sort_unstable();
        record(out, "server/ping_rtt", "p50_us", percentile_us(&samples, 50.0), "µs");
        record(out, "server/ping_rtt", "p99_us", percentile_us(&samples, 99.0), "µs");
        record(out, "server/ping_rtt", "p99.9_us", percentile_us(&samples, 99.9), "µs");
    }

    // Pipelined SET/GET over one connection, batch of 128.
    const BATCH: usize = 128;
    let distinct = 10_000usize;
    let key = |i: usize| format!("k{:07}", i % distinct);
    {
        let dur = Duration::from_secs_f64(secs);
        let t = Instant::now();
        let mut done = 0usize;
        while t.elapsed() < dur {
            let mut buf = Vec::with_capacity(BATCH * 46);
            for j in 0..BATCH {
                buf.extend_from_slice(
                    format!("*3\r\n$3\r\nSET\r\n$8\r\n{}\r\n$2\r\nvv\r\n", key(done + j)).as_bytes(),
                );
            }
            s.write_all(&buf).unwrap();
            let mut rbuf = vec![0u8; BATCH * 5];
            s.read_exact(&mut rbuf).unwrap();
            done += BATCH;
        }
        record(out, "server/pipeline_set", "throughput", done as f64 / t.elapsed().as_secs_f64(), "ops/s");
    }
    {
        let dur = Duration::from_secs_f64(secs);
        let t = Instant::now();
        let mut done = 0usize;
        while t.elapsed() < dur {
            let mut buf = Vec::with_capacity(BATCH * 38);
            for j in 0..BATCH {
                buf.extend_from_slice(
                    format!("*2\r\n$3\r\nGET\r\n$8\r\n{}\r\n", key(done + j)).as_bytes(),
                );
            }
            s.write_all(&buf).unwrap();
            let mut rbuf = vec![0u8; BATCH * 8]; // "$2\r\nvv\r\n" = 8 bytes
            s.read_exact(&mut rbuf).unwrap();
            done += BATCH;
        }
        record(out, "server/pipeline_get", "throughput", done as f64 / t.elapsed().as_secs_f64(), "ops/s");
    }
}

// ------------------------------------------------------------ sim suite

fn sim_suite(out: &mut Vec<BenchResult>) {
    println!("sim (deterministic policies):");
    let trace = zipf_trace(10_000, 200_000, 1.0, 0xc0ff_ee);
    let cap = 2_000;
    {
        let t = Instant::now();
        let r = run_policy(&trace, cap, &mut Lru::new(cap));
        let s = t.elapsed().as_secs_f64();
        record(out, "sim/policy_lru", "throughput", r.ops as f64 / s, "ops/s");
        record(out, "sim/policy_lru", "hit_ratio", r.hit_ratio, "ratio");
    }
    {
        let t = Instant::now();
        let r = run_policy(&trace, cap, &mut Belady::new(cap));
        let s = t.elapsed().as_secs_f64();
        record(out, "sim/policy_belady", "throughput", r.ops as f64 / s, "ops/s");
        record(out, "sim/policy_belady", "hit_ratio", r.hit_ratio, "ratio");
    }
}

// ------------------------------------------------------------ registry

fn write_jsonl(path: &str, phase: &str, results: &[BenchResult], cores: usize) -> std::io::Result<()> {
    let p = std::path::Path::new(path);
    if let Some(dir) = p.parent() {
        if !dir.as_os_str().is_empty() {
            std::fs::create_dir_all(dir)?;
        }
    }
    let ts = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs();
    let mut f = OpenOptions::new().create(true).append(true).open(p)?;
    for r in results {
        writeln!(
            f,
            "{{\"ts\":{ts},\"phase\":\"{}\",\"bench\":\"{}\",\"metric\":\"{}\",\"value\":{:.3},\"unit\":\"{}\",\"cores\":{}}}",
            phase, r.bench, r.metric, r.value, r.unit, cores
        )?;
    }
    Ok(())
}

fn main() {
    let args = parse_args();
    let cores = std::thread::available_parallelism().map(|n| n.get()).unwrap_or(0);
    let profile = if cfg!(debug_assertions) { "debug" } else { "release" };
    println!(
        "meridian-bench — phase {} — {} profile — {cores} cores — {:.1}s per bench",
        args.phase, profile, args.secs
    );

    let mut out = Vec::new();
    match args.suite.as_str() {
        "engine" => engine_suite(&mut out, args.secs),
        "server" => server_suite(&mut out, args.secs),
        "sim" => sim_suite(&mut out),
        "all" => {
            engine_suite(&mut out, args.secs);
            server_suite(&mut out, args.secs);
            sim_suite(&mut out);
        }
        other => {
            eprintln!("unknown suite '{other}' (engine|server|sim|all)");
            std::process::exit(2);
        }
    }

    match write_jsonl(&args.jsonl, &args.phase, &out, cores) {
        Ok(()) => println!("\nresults appended to {}", args.jsonl),
        Err(e) => eprintln!("\nwarning: could not write {}: {e}", args.jsonl),
    }
}
