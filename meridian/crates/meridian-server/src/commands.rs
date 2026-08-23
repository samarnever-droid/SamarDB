//! Command dispatch: the Redis-compatible subset plus the native MD.* API.

use std::time::Duration;
use meridian_core::{
    apply_delta, Engine, SetOpts, SetOutcome, Slo, TtlStatus, DeltaOp,
};
use meridian_proto::Frame;

pub enum Action {
    Reply(Frame),
    Quit,
}

pub fn dispatch(e: &Engine, proto3: &mut bool, args: Vec<Vec<u8>>) -> Action {
    if args.is_empty() {
        return Action::Reply(Frame::Null);
    }
    let a0 = &args[0];
    let eq = |s: &[u8]| a0.eq_ignore_ascii_case(s);
    if eq(b"PING") {
        Action::Reply(match args.len() {
            1 => Frame::Simple("PONG".into()),
            2 => Frame::Bulk(args[1].clone()),
            _ => Frame::Error("ERR wrong number of arguments for 'ping'".into()),
        })
    } else if eq(b"ECHO") {
        if args.len() != 2 {
            er("ERR wrong number of arguments for 'echo'")
        } else {
            Action::Reply(Frame::Bulk(args[1].clone()))
        }
    } else if eq(b"HELLO") {
        if let Some(v) = sarg(&args, 1) {
            match v.as_str() {
                "2" => *proto3 = false,
                "3" => *proto3 = true,
                "AUTH" | "SETNAME" | "SETINFO" => {
                    return er("ERR HELLO option not supported")
                }
                _ => return er("ERR NOPROTO unsupported protocol version"),
            }
        }
        Action::Reply(Frame::Map(vec![
            (Frame::Bulk(b"server".to_vec()), Frame::Bulk(b"meridian".to_vec())),
            (Frame::Bulk(b"version".to_vec()), Frame::Bulk(env!("CARGO_PKG_VERSION").as_bytes().to_vec())),
            (Frame::Bulk(b"proto".to_vec()), Frame::Int(if *proto3 { 3 } else { 2 })),
        ]))
    } else if eq(b"COMMAND") {
        Action::Reply(Frame::Array(vec![]))
    } else if eq(b"CONFIG") {
        if sarg(&args, 1).map(|s| s.eq_ignore_ascii_case("GET")).unwrap_or(false) {
            Action::Reply(Frame::Array(vec![]))
        } else {
            Action::Reply(Frame::Simple("OK".into()))
        }
    } else if eq(b"CLIENT") {
        Action::Reply(Frame::Simple("OK".into()))
    } else if eq(b"QUIT") {
        Action::Quit
    } else if eq(b"GET") {
        if args.len() != 2 {
            er("ERR wrong number of arguments for 'get'")
        } else {
            Action::Reply(match e.get_l0(&args[1]) {
                Some(v) => Frame::Bulk(v),
                None => Frame::Null,
            })
        }
    } else if eq(b"MGET") {
        if args.len() < 2 {
            er("ERR wrong number of arguments for 'mget'")
        } else {
            Action::Reply(Frame::Array(
                args[1..].iter().map(|k| e.get_l0(k).map(Frame::Bulk).unwrap_or(Frame::Null)).collect(),
            ))
        }
    } else if eq(b"MSET") {
        if args.len() < 3 || args.len() % 2 != 1 {
            er("ERR wrong number of arguments for 'mset'")
        } else {
            for pair in args[1..].chunks(2) {
                e.set(&pair[0], &pair[1]);
            }
            Action::Reply(Frame::Simple("OK".into()))
        }
    } else if eq(b"SCAN") {
        scan_cmd(e, &args)
    } else if eq(b"SET") {
        set_cmd(e, &args)
    } else if eq(b"DEL") {
        let n = args[1..].iter().filter(|k| e.del(k)).count();
        Action::Reply(Frame::Int(n as i64))
    } else if eq(b"EXISTS") {
        let n = args[1..].iter().filter(|k| e.exists(k)).count();
        Action::Reply(Frame::Int(n as i64))
    } else if eq(b"EXPIRE") {
        if args.len() != 3 {
            er("ERR wrong number of arguments for 'expire'")
        } else {
            match num(&args[2]) {
                Some(secs) => Action::Reply(Frame::Int(e.expire(&args[1], Some(Duration::from_secs(secs))) as i64)),
                None => er("ERR value is not an integer or out of range"),
            }
        }
    } else if eq(b"TTL") {
        if args.len() != 2 {
            er("ERR wrong number of arguments for 'ttl'")
        } else {
            let v = match e.ttl(&args[1]) {
                TtlStatus::Missing => -2,
                TtlStatus::Persistent => -1,
                TtlStatus::Expires(ms) => {
                    let secs = ms / 1000;
                    (if secs == 0 && ms > 0 { 1 } else { secs }) as i64
                }
            };
            Action::Reply(Frame::Int(v))
        }
    } else if eq(b"PTTL") {
        if args.len() != 2 {
            er("ERR wrong number of arguments for 'pttl'")
        } else {
            let v = match e.ttl(&args[1]) {
                TtlStatus::Missing => -2,
                TtlStatus::Persistent => -1,
                TtlStatus::Expires(ms) => ms as i64,
            };
            Action::Reply(Frame::Int(v))
        }
    } else if eq(b"DBSIZE") {
        Action::Reply(Frame::Int(e.item_count() as i64))
    } else if eq(b"FLUSHALL") || eq(b"FLUSHDB") {
        e.flush();
        Action::Reply(Frame::Simple("OK".into()))
    } else if eq(b"INFO") {
        Action::Reply(Frame::Bulk(info_text(e).into_bytes()))
    } else if eq(b"MD.STATS") {
        Action::Reply(stats_frame(&e.stats()))
    } else if eq(b"MD.SLO") {
        md_slo(e, &args)
    } else if eq(b"MD.MAINTAIN") {
        md_maintain(e, &args)
    } else if eq(b"MD.INVALIDATE") {
        md_invalidate(e, &args)
    } else if eq(b"MD.HELP") {
        Action::Reply(Frame::Bulk(
            "MERIDIAN native API:\n\
             MD.STATS\n\
             MD.SLO SET <class> [freshness_p99_ms=..] [origin_qps_max=..] [latency_p99_us=..] [priority=..]\n\
             MD.SLO GET <class> | MD.SLO DEL <class> | MD.SLO LIST\n\
             MD.MAINTAIN <key> <SUM|COUNT|GROUPBY> <delta>\n\
             MD.INVALIDATE <key...>\n"
                .as_bytes()
                .to_vec(),
        ))
    } else {
        Action::Reply(Frame::Error(format!(
            "ERR unknown command '{}'",
            String::from_utf8_lossy(&args[0])
        )))
    }
}

fn md_maintain(e: &Engine, args: &[Vec<u8>]) -> Action {
    if args.len() < 4 {
        return er("ERR MD.MAINTAIN <key> <SUM|COUNT|GROUPBY> <delta>");
    }
    let key = &args[1];
    let op_name = String::from_utf8_lossy(&args[2]).to_ascii_uppercase();
    let delta_s = String::from_utf8_lossy(&args[3]);
    let delta: i64 = delta_s.parse().unwrap_or(0);

    let cur = e.get(key).unwrap_or_default();
    let updated = match op_name.as_str() {
        "SUM" => apply_delta(&cur, &DeltaOp::Sum { delta }),
        "COUNT" => apply_delta(&cur, &DeltaOp::Count { delta }),
        "GROUPBY" => {
            let grp = sarg(args, 4).unwrap_or_else(|| "default".to_string());
            apply_delta(&cur, &DeltaOp::GroupBy { group: grp, delta })
        }
        _ => return er("ERR unknown delta operation"),
    };
    e.set(key, &updated);
    Action::Reply(Frame::Simple("OK".into()))
}

fn md_invalidate(e: &Engine, args: &[Vec<u8>]) -> Action {
    if args.len() < 2 {
        return er("ERR MD.INVALIDATE <key...>");
    }
    let mut count = 0;
    for k in &args[1..] {
        if e.del(k) {
            count += 1;
        }
    }
    Action::Reply(Frame::Int(count))
}

fn scan_cmd(e: &Engine, args: &[Vec<u8>]) -> Action {
    let Some(cursor_s) = sarg(args, 1) else {
        return er("ERR wrong number of arguments for 'scan'");
    };
    let Ok(cursor) = cursor_s.parse::<u64>() else {
        return er("ERR invalid cursor");
    };
    let mut count: usize = 10;
    let mut pattern: Option<Vec<u8>> = None;
    let mut i = 2;
    while i < args.len() {
        let a = &args[i];
        if a.eq_ignore_ascii_case(b"MATCH") {
            pattern = args.get(i + 1).cloned();
            if pattern.is_none() {
                return er("ERR syntax error");
            }
            i += 2;
        } else if a.eq_ignore_ascii_case(b"COUNT") {
            let Some(c) = args.get(i + 1).and_then(|b| num(b)) else {
                return er("ERR syntax error");
            };
            count = c as usize;
            i += 2;
        } else {
            return er("ERR syntax error");
        }
    }
    let (next_cursor, res) = e.scan_from(cursor, count, pattern.as_deref());
    Action::Reply(Frame::Array(vec![
        Frame::Bulk(next_cursor.to_string().into_bytes()),
        Frame::Array(res.into_iter().map(Frame::Bulk).collect()),
    ]))
}

fn set_cmd(e: &Engine, args: &[Vec<u8>]) -> Action {
    if args.len() < 3 {
        return er("ERR wrong number of arguments for 'set'");
    }
    let mut o = SetOpts::default();
    let mut get = false;
    let mut i = 3;
    while i < args.len() {
        let opt = &args[i];
        if opt.eq_ignore_ascii_case(b"EX") {
            let Some(secs) = args.get(i + 1).and_then(|b| num(b)) else {
                return er("ERR syntax error");
            };
            o.ttl = Some(Duration::from_secs(secs));
            i += 2;
        } else if opt.eq_ignore_ascii_case(b"PX") {
            let Some(ms) = args.get(i + 1).and_then(|b| num(b)) else {
                return er("ERR syntax error");
            };
            o.ttl = Some(Duration::from_millis(ms));
            i += 2;
        } else if opt.eq_ignore_ascii_case(b"NX") {
            o.nx = true;
            i += 1;
        } else if opt.eq_ignore_ascii_case(b"XX") {
            o.xx = true;
            i += 1;
        } else if opt.eq_ignore_ascii_case(b"KEEPTTL") {
            o.keepttl = true;
            i += 1;
        } else if opt.eq_ignore_ascii_case(b"GET") {
            get = true;
            i += 1;
        } else {
            return er("ERR syntax error");
        }
    }
    o.get_old = get;
    match e.set_opts(&args[1], &args[2], &o) {
        SetOutcome::Stored(old) => Action::Reply(match (get, old) {
            (true, Some(v)) => Frame::Bulk(v),
            (true, None) => Frame::Null,
            _ => Frame::Simple("OK".into()),
        }),
        SetOutcome::NotStored => Action::Reply(Frame::Null),
    }
}

fn md_slo(e: &Engine, args: &[Vec<u8>]) -> Action {
    let Some(sub) = sarg(args, 1).map(|s| s.to_ascii_uppercase()) else {
        return er("ERR MD.SLO SET|GET|DEL|LIST ...");
    };
    match sub.as_str() {
        "SET" => {
            let Some(class) = sarg(args, 2) else {
                return er("ERR MD.SLO SET <class> [k=v ...]");
            };
            let mut slo = e.slo_get(&class).unwrap_or(Slo {
                class: class.clone(),
                freshness_p99_ms: 250,
                origin_qps_max: 1000,
                latency_p99_us: 2500,
                priority: 3,
            });
            for a in &args[3..] {
                let s = String::from_utf8_lossy(a);
                let Some((k, v)) = s.split_once('=') else {
                    return er("ERR MD.SLO fields are k=v pairs");
                };
                match k {
                    "freshness_p99_ms" => slo.freshness_p99_ms = v.parse().unwrap_or(slo.freshness_p99_ms),
                    "origin_qps_max" => slo.origin_qps_max = v.parse().unwrap_or(slo.origin_qps_max),
                    "latency_p99_us" => slo.latency_p99_us = v.parse().unwrap_or(slo.latency_p99_us),
                    "priority" => slo.priority = v.parse().unwrap_or(slo.priority),
                    _ => return er(format!("ERR unknown SLO field '{k}'")),
                }
            }
            e.slo_set(slo);
            Action::Reply(Frame::Simple("OK".into()))
        }
        "GET" => {
            let Some(class) = sarg(args, 2) else {
                return er("ERR MD.SLO GET <class>");
            };
            match e.slo_get(&class) {
                Some(s) => Action::Reply(slo_frame(&s)),
                None => er(format!("ERR no SLO class '{class}'")),
            }
        }
        "DEL" => {
            let Some(class) = sarg(args, 2) else {
                return er("ERR MD.SLO DEL <class>");
            };
            Action::Reply(Frame::Int(e.slo_del(&class) as i64))
        }
        "LIST" => {
            Action::Reply(Frame::Array(e.slo_list().iter().map(slo_frame).collect()))
        }
        _ => er("ERR MD.SLO SET|GET|DEL|LIST ..."),
    }
}

fn slo_frame(s: &Slo) -> Frame {
    Frame::Map(vec![
        (Frame::Bulk(b"class".to_vec()), Frame::Bulk(s.class.as_bytes().to_vec())),
        (Frame::Bulk(b"freshness_p99_ms".to_vec()), Frame::Int(s.freshness_p99_ms as i64)),
        (Frame::Bulk(b"origin_qps_max".to_vec()), Frame::Int(s.origin_qps_max as i64)),
        (Frame::Bulk(b"latency_p99_us".to_vec()), Frame::Int(s.latency_p99_us as i64)),
        (Frame::Bulk(b"priority".to_vec()), Frame::Int(s.priority as i64)),
    ])
}

fn stats_frame(st: &meridian_core::EngineStats) -> Frame {
    let i = |k: &str, v: u64| (Frame::Bulk(k.as_bytes().to_vec()), Frame::Int(v as i64));
    Frame::Map(vec![
        i("shards", st.shards),
        i("items", st.items),
        i("hits", st.hits),
        i("misses", st.misses),
        (
            Frame::Bulk(b"hit_ratio".to_vec()),
            Frame::Bulk(format!("{:.4}", st.hit_ratio).into_bytes()),
        ),
        i("expired", st.expired),
        i("evictions", st.evictions),
        i("sets", st.sets),
        i("dels", st.dels),
        i("seqlock_retries", st.retries),
        i("uptime_ms", st.uptime_ms),
    ])
}

fn info_text(e: &Engine) -> String {
    let st = e.stats();
    format!(
        "# meridian\r\n\
         meridian_version:{}\r\n\
         shards:{}\r\n\
         items:{}\r\n\
         hits:{}\r\n\
         misses:{}\r\n\
         hit_ratio:{:.4}\r\n\
         expired:{}\r\n\
         evictions:{}\r\n\
         sets:{}\r\n\
         dels:{}\r\n\
         seqlock_retries:{}\r\n\
         uptime_ms:{}\r\n",
        env!("CARGO_PKG_VERSION"),
        st.shards,
        st.items,
        st.hits,
        st.misses,
        st.hit_ratio,
        st.expired,
        st.evictions,
        st.sets,
        st.dels,
        st.retries,
        st.uptime_ms,
    )
}

fn sarg(args: &[Vec<u8>], i: usize) -> Option<String> {
    args.get(i).map(|a| String::from_utf8_lossy(a).into_owned())
}

fn num(b: &[u8]) -> Option<u64> {
    std::str::from_utf8(b).ok()?.parse().ok()
}

fn er(msg: impl Into<String>) -> Action {
    Action::Reply(Frame::Error(msg.into()))
}
