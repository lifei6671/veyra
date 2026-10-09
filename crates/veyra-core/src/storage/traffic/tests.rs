//! 每项测试保护真实 SQLite 的用户统计契约；无 Mock、无真实 child。
use super::*;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);
struct Directory(PathBuf);
impl Directory {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "veyra-p3-04-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir_all(&path).unwrap();
        Self(path)
    }
    fn db(&self) -> Connection {
        Connection::open(self.0.join("traffic.sqlite3")).unwrap()
    }
}
impl Drop for Directory {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}
fn source(instance: &str) -> Source {
    Source {
        epoch: "epoch-1".into(),
        instance: instance.into(),
    }
}
fn sample(key: &str, end: i64, count: Count) -> Sample {
    Sample {
        source: source("owned-child"),
        key: key.into(),
        start_ms: end - 1000,
        end_ms: end,
        count,
    }
}
fn interval(key: &str, end: i64, up: u64) -> Sample {
    sample(
        key,
        end,
        Count::ObservedInterval {
            upload: up,
            download: up * 2,
        },
    )
}
fn dimensions(host: &str) -> Dimensions {
    Dimensions {
        node: Some("node-a".into()),
        host: Some(host.into()),
        client: Some("client-a".into()),
        direct: Some(false),
    }
}
fn connection(key: &str, end: i64, up: u64, host: &str) -> Sample {
    sample(
        key,
        end,
        Count::Connection {
            id: "connection-a".into(),
            upload: up,
            download: up * 2,
            dimensions: dimensions(host),
        },
    )
}
fn scalar(db: &Connection, sql: &str) -> i64 {
    db.query_row(sql, [], |row| row.get(0)).unwrap()
}
const NOW: i64 = 1_791_504_000_000;

#[test]
fn sqlite_reopen_interval_bytes_are_not_differenced_and_replay_is_idempotent() {
    // P0 traffic 区间 bytes 原样累加，持久化去重跨 writer 重建仍有效。
    let dir = Directory::new();
    let writer = TrafficWriter::open(&dir.0, chrono_tz::UTC).unwrap();
    writer.submit(interval("one", NOW, 100)).unwrap();
    writer.submit(interval("two", NOW + 1000, 80)).unwrap();
    writer.submit(interval("two", NOW + 1000, 80)).unwrap();
    writer.close().unwrap();
    let writer = TrafficWriter::open(&dir.0, chrono_tz::UTC).unwrap();
    writer.submit(interval("two", NOW + 1000, 80)).unwrap();
    writer.submit(interval("three", NOW + 2000, 20)).unwrap();
    writer.close().unwrap();
    let db = dir.db();
    assert_eq!(scalar(&db, "SELECT SUM(up) FROM detail"), 200);
    assert_eq!(scalar(&db, "SELECT SUM(down) FROM daily"), 400);
    assert_eq!(scalar(&db, "SELECT COUNT(*) FROM detail"), 3);
    assert_eq!(scalar(&db, "PRAGMA user_version"), 1);
}

#[test]
fn cumulative_connection_reset_dimension_change_and_restart_keep_proven_attribution() {
    // 起始累计不冒充新流量，重复/逆序不多计，维度切换未证实区间单列。
    let dir = Directory::new();
    let writer = TrafficWriter::open(&dir.0, chrono_tz::UTC).unwrap();
    for s in [
        connection("baseline", NOW, 100, "old.example"),
        connection("delta", NOW + 1000, 150, "old.example"),
        connection("duplicate", NOW + 1000, 150, "old.example"),
        connection("out-of-order", NOW + 500, 140, "old.example"),
        connection("switch", NOW + 2000, 180, "new.example"),
        connection("new-delta", NOW + 3000, 200, "new.example"),
        connection("reset", NOW + 4000, 5, "new.example"),
        connection("reset-delta", NOW + 5000, 15, "new.example"),
    ] {
        writer.submit(s).unwrap();
    }
    writer.close().unwrap();
    let writer = TrafficWriter::open(&dir.0, chrono_tz::UTC).unwrap();
    writer
        .submit(connection("reopen", NOW + 6000, 25, "new.example"))
        .unwrap();
    let mut restart = connection("new-instance", NOW + 7000, 1000, "new.example");
    restart.source = source("restart-child");
    writer.submit(restart).unwrap();
    writer.close().unwrap();
    let db = dir.db();
    assert_eq!(
        scalar(&db, "SELECT SUM(up) FROM detail WHERE kind='attributed'"),
        90
    );
    assert_eq!(
        scalar(&db, "SELECT SUM(up) FROM detail WHERE host='old.example'"),
        50
    );
    assert_eq!(
        scalar(&db, "SELECT SUM(up) FROM detail WHERE host='new.example'"),
        40
    );
    assert_eq!(
        scalar(
            &db,
            "SELECT SUM(up) FROM detail WHERE kind='dimension_gap' AND host IS NULL AND client IS NULL"
        ),
        30
    );
    assert_eq!(
        scalar(
            &db,
            "SELECT COUNT(*) FROM detail WHERE kind='counter_reset' AND up=0"
        ),
        1
    );
    assert_eq!(scalar(&db, "SELECT SUM(connections) FROM daily"), 2);
}

#[test]
fn epoch_and_instance_keys_isolate_identical_sequences() {
    // 不同受管 child/epoch 的相同计数键互不覆盖，也不混入其他实例。
    let dir = Directory::new();
    let writer = TrafficWriter::open(&dir.0, chrono_tz::UTC).unwrap();
    let a = interval("same", NOW, 10);
    let mut b = a.clone();
    b.source.instance = "other-child".into();
    let mut c = a.clone();
    c.source.epoch = "epoch-2".into();
    for s in [a, b, c] {
        writer.submit(s).unwrap();
    }
    writer.close().unwrap();
    let db = dir.db();
    assert_eq!(scalar(&db, "SELECT COUNT(*) FROM daily"), 3);
    assert_eq!(scalar(&db, "SELECT SUM(up) FROM daily"), 30);
}

#[test]
fn unattributed_and_observed_remain_separate_without_total_minus_attributed() {
    // 不同来源口径不得相减拼平，也不虚构 host/client。
    let dir = Directory::new();
    let writer = TrafficWriter::open(&dir.0, chrono_tz::UTC).unwrap();
    writer.submit(interval("observed", NOW, 10)).unwrap();
    writer
        .submit(sample(
            "unknown",
            NOW,
            Count::Unattributed {
                upload: 40,
                download: 80,
            },
        ))
        .unwrap();
    writer.close().unwrap();
    let db = dir.db();
    assert_eq!(
        scalar(&db, "SELECT SUM(up) FROM daily WHERE kind='observed'"),
        10
    );
    assert_eq!(
        scalar(&db, "SELECT SUM(up) FROM daily WHERE kind='unattributed'"),
        40
    );
    assert_eq!(
        scalar(&db, "SELECT COUNT(*) FROM detail WHERE host IS NOT NULL"),
        0
    );
}

#[test]
fn timezone_handles_dst_short_long_days_and_year_boundary() {
    // 统计日期按 IANA 本地日，不把一天假定为24小时；保留 UTC 原始区间。
    let zone = chrono_tz::America::New_York;
    let stamp = |y, m, d, h| {
        Utc.with_ymd_and_hms(y, m, d, h, 0, 0)
            .unwrap()
            .timestamp_millis()
    };
    let spring_start = stamp(2026, 3, 8, 5);
    let spring_end = stamp(2026, 3, 9, 4);
    assert_eq!(spring_end - spring_start, 23 * 3_600_000);
    assert_eq!(statistical_day(spring_start, zone).unwrap(), "2026-03-08");
    assert_eq!(statistical_day(spring_end - 1, zone).unwrap(), "2026-03-08");
    assert_eq!(statistical_day(spring_end, zone).unwrap(), "2026-03-09");
    let fall_start = stamp(2026, 11, 1, 4);
    let fall_end = stamp(2026, 11, 2, 5);
    assert_eq!(fall_end - fall_start, 25 * 3_600_000);
    assert_eq!(statistical_day(fall_end - 1, zone).unwrap(), "2026-11-01");
    let dir = Directory::new();
    let writer = TrafficWriter::open(&dir.0, zone).unwrap();
    let end = stamp(2027, 1, 1, 5);
    writer.submit(interval("old-year", end - 1, 7)).unwrap();
    writer.submit(interval("new-year", end, 9)).unwrap();
    writer.close().unwrap();
    let db = dir.db();
    assert_eq!(
        scalar(&db, "SELECT SUM(up) FROM daily WHERE day='2026-12-31'"),
        7
    );
    assert_eq!(
        scalar(&db, "SELECT SUM(up) FROM daily WHERE day='2027-01-01'"),
        9
    );
    assert!(TrafficWriter::open(&dir.0, chrono_tz::UTC).is_err());
}

#[test]
fn failing_batch_rolls_back_without_overwriting_old_statistics() {
    // 真 SQLite trigger 模拟磁盘写失败，批事务整体回滚并显式失败。
    let dir = Directory::new();
    let writer = TrafficWriter::open(&dir.0, chrono_tz::UTC).unwrap();
    writer.submit(interval("old", NOW, 11)).unwrap();
    writer.flush().unwrap();
    dir.db().execute_batch("CREATE TRIGGER reject_bad BEFORE INSERT ON detail WHEN NEW.key='bad' BEGIN SELECT RAISE(ABORT,'controlled failure'); END;").unwrap();
    writer
        .submit(interval("good-in-failed-batch", NOW + 1000, 20))
        .unwrap();
    writer.submit(interval("bad", NOW + 2000, 30)).unwrap();
    assert!(writer.flush().is_err());
    let status = writer.status();
    assert!(status.failed);
    assert_eq!(status.failed_batch, 2);
    assert!(writer.close().is_err());
    let db = dir.db();
    assert_eq!(scalar(&db, "SELECT SUM(up) FROM daily"), 11);
    assert_eq!(scalar(&db, "SELECT COUNT(*) FROM seen"), 1);
}

#[test]
fn bounded_queue_reports_backpressure_while_sqlite_is_locked() {
    // 外部锁住真 DB 时队列不会无界增长，掉队可见；解锁后已接收项落盘。
    let dir = Directory::new();
    let writer = TrafficWriter::open(&dir.0, chrono_tz::UTC).unwrap();
    let lock = dir.db();
    lock.execute_batch("BEGIN IMMEDIATE").unwrap();
    let mut full = false;
    let mut accepted = 0;
    for i in 0..4096 {
        match writer.submit(interval(&format!("locked-{i}"), NOW, 1)) {
            Ok(()) => accepted += 1,
            Err(Error::Full) => {
                full = true;
                break;
            }
            Err(e) => panic!("unexpected: {e}"),
        }
    }
    assert!(full);
    assert!(writer.status().rejected >= 1);
    lock.execute_batch("ROLLBACK").unwrap();
    writer.flush().unwrap();
    writer.close().unwrap();
    assert_eq!(scalar(&dir.db(), "SELECT COUNT(*) FROM detail"), accepted);
}

#[test]
fn retention_prunes_old_detail_preserves_daily_and_rejects_old_replay() {
    // 细节7天到期后保留长期日总量，过期帧不能重新加到旧统计。
    let dir = Directory::new();
    let writer = TrafficWriter::open(&dir.0, chrono_tz::UTC).unwrap();
    writer.submit(interval("old", NOW, 11)).unwrap();
    writer.flush().unwrap();
    writer
        .submit(interval("current", NOW + 8 * 86_400_000, 17))
        .unwrap();
    writer.flush().unwrap();
    assert_eq!(scalar(&dir.db(), "SELECT COUNT(*) FROM detail"), 1);
    assert_eq!(scalar(&dir.db(), "SELECT SUM(up) FROM daily"), 28);
    assert!(writer.status().detail_cutoff_ms.unwrap() > NOW);
    writer.submit(interval("old", NOW, 11)).unwrap();
    assert!(writer.flush().is_err());
    assert!(writer.close().is_err());
    assert_eq!(scalar(&dir.db(), "SELECT SUM(up) FROM daily"), 28);
}

#[test]
fn capacity_failure_is_visible_and_does_not_silently_drop_associations() {
    // 小预算不是 TopN 截断；无法满足时失败，旧事务仍完整。
    let dir = Directory::new();
    let writer = TrafficWriter::open_with_capacity(&dir.0, chrono_tz::UTC, 1).unwrap();
    writer.submit(interval("too-large", NOW, 100)).unwrap();
    assert!(writer.flush().is_err());
    let status = writer.status();
    assert!(status.failed);
    assert!(status.capacity_blocked);
    assert!(writer.close().is_err());
    assert_eq!(scalar(&dir.db(), "SELECT COUNT(*) FROM daily"), 0);
}

#[test]
fn high_cardinality_real_sqlite_measurement_preserves_5000_actual_combinations() {
    // 5000真实出现的连接组合逐条保存，无交叉笛卡尔积/TopN。输出实际空间与性能。
    let dir = Directory::new();
    let writer = TrafficWriter::open(&dir.0, chrono_tz::UTC).unwrap();
    let begin = Instant::now();
    for i in 0..5000 {
        let dims = Dimensions {
            node: Some(format!("node-{}", i % 20)),
            host: Some(format!("host-{i}.example")),
            client: Some(format!("client-{i}")),
            direct: Some(i % 7 == 0),
        };
        for (suffix, end, upload) in [("base", NOW, 100), ("delta", NOW + 1000, 200)] {
            writer
                .submit(sample(
                    &format!("{i}-{suffix}"),
                    end,
                    Count::Connection {
                        id: format!("connection-{i}"),
                        upload,
                        download: upload * 2,
                        dimensions: dims.clone(),
                    },
                ))
                .unwrap();
        }
        if i % 64 == 63 {
            writer.flush().unwrap();
        }
    }
    writer.flush().unwrap();
    let write_time = begin.elapsed();
    writer.close().unwrap();
    let db = dir.db();
    assert_eq!(
        scalar(&db, "SELECT COUNT(*) FROM detail WHERE kind='attributed'"),
        5000
    );
    assert_eq!(
        scalar(
            &db,
            "SELECT COUNT(DISTINCT host) FROM detail WHERE kind='attributed'"
        ),
        5000
    );
    assert_eq!(
        scalar(&db, "SELECT SUM(up) FROM detail WHERE kind='attributed'"),
        500000
    );
    let plan:String=db.query_row("EXPLAIN QUERY PLAN SELECT SUM(up) FROM detail WHERE epoch='epoch-1' AND instance='owned-child' AND bucket_ms=?",[NOW/BUCKET_MS*BUCKET_MS],|r|r.get(3)).unwrap();
    assert!(plan.contains("detail_window"), "{plan}");
    let read_begin = Instant::now();
    for _ in 0..100 {
        let _: i64 = db
            .query_row(
                "SELECT SUM(up) FROM detail WHERE epoch=? AND instance=? AND bucket_ms=?",
                params!["epoch-1", "owned-child", NOW / BUCKET_MS * BUCKET_MS],
                |r| r.get(0),
            )
            .unwrap();
    }
    let query_us = read_begin.elapsed().as_micros() / 100;
    let usage = usage(&dir.0.join("traffic.sqlite3")).unwrap();
    assert_eq!(usage.database_bytes, usage.page_count * usage.page_size);
    assert_eq!(usage.wal_bytes, 0);
    assert_eq!(usage.shm_bytes, 0);
    println!(
        "P3_04_MEASUREMENT samples=10000 combinations=5000 database_bytes={} page_count={} freelist={} page_size={} write_ms={} samples_per_second={:.0} indexed_query_us={} daily_5000_per_minute_projection_bytes={} query_plan={plan}",
        usage.database_bytes,
        usage.page_count,
        usage.freelist_count,
        usage.page_size,
        write_time.as_millis(),
        10000.0 / write_time.as_secs_f64(),
        query_us,
        usage.database_bytes * 1440
    );
}

#[test]
fn incomplete_dimensions_and_one_direction_reset_are_explicit() {
    // 不完整身份保留已知字段并标记partial；单方向reset不丢另一方向已确认增量。
    let dir = Directory::new();
    let writer = TrafficWriter::open(&dir.0, chrono_tz::UTC).unwrap();
    for (key, end, up, down) in [
        ("base", NOW, 100, 100),
        ("partial", NOW + 1000, 120, 130),
        ("reset-up", NOW + 2000, 1, 160),
    ] {
        writer
            .submit(sample(
                key,
                end,
                Count::Connection {
                    id: "partial-connection".into(),
                    upload: up,
                    download: down,
                    dimensions: Dimensions {
                        host: Some("known.example".into()),
                        ..Dimensions::default()
                    },
                },
            ))
            .unwrap();
    }
    writer.close().unwrap();
    let db = dir.db();
    assert_eq!(
        scalar(
            &db,
            "SELECT SUM(up) FROM detail WHERE kind='partial' AND host='known.example' AND client IS NULL"
        ),
        20
    );
    assert_eq!(
        scalar(
            &db,
            "SELECT SUM(down) FROM detail WHERE kind='counter_reset' AND up=0"
        ),
        30
    );
    assert_eq!(scalar(&db, "SELECT SUM(down) FROM daily"), 60);
}

#[test]
fn retention_bounds_baselines_dedupe_and_long_term_summary() {
    // 清理同时覆盖counter/去重和一年日汇总，旧连接再出现只建基线，不差分跨保留期。
    let dir = Directory::new();
    let writer = TrafficWriter::open(&dir.0, chrono_tz::UTC).unwrap();
    writer
        .submit(connection("old-baseline", NOW, 100, "known.example"))
        .unwrap();
    writer.submit(interval("old-traffic", NOW, 11)).unwrap();
    writer.flush().unwrap();
    writer
        .submit(interval("after-year", NOW + 400 * 86_400_000, 17))
        .unwrap();
    writer.flush().unwrap();
    let db = dir.db();
    assert_eq!(scalar(&db, "SELECT COUNT(*) FROM counters"), 0);
    assert_eq!(scalar(&db, "SELECT COUNT(*) FROM seen"), 1);
    assert_eq!(scalar(&db, "SELECT SUM(up) FROM daily"), 17);
    writer
        .submit(connection(
            "reappeared",
            NOW + 400 * 86_400_000 + 1000,
            9999,
            "known.example",
        ))
        .unwrap();
    writer.close().unwrap();
    assert_eq!(scalar(&db, "SELECT SUM(up) FROM daily"), 17);
    assert_eq!(scalar(&db, "SELECT COUNT(*) FROM counters"), 1);
}

#[test]
fn connection_presence_is_distinct_per_bucket_and_local_day() {
    // 同一稳定connection每桶/每统计日只计一次；跨年跨日继续存在仍计入新日。
    let dir = Directory::new();
    let writer = TrafficWriter::open(&dir.0, chrono_tz::UTC).unwrap();
    let midnight = Utc
        .with_ymd_and_hms(2027, 1, 1, 0, 0, 0)
        .unwrap()
        .timestamp_millis();
    for (key, end, up) in [
        ("first-bucket", midnight - 120_000, 100),
        ("same-bucket", midnight - 119_000, 110),
        ("second-bucket", midnight - 60_000, 120),
        ("new-day", midnight, 130),
        ("same-new-bucket", midnight + 1000, 140),
    ] {
        writer
            .submit(connection(key, end, up, "stable.example"))
            .unwrap();
    }
    writer.close().unwrap();
    let db = dir.db();
    assert_eq!(scalar(&db, "SELECT SUM(connections) FROM detail"), 3);
    assert_eq!(
        scalar(
            &db,
            "SELECT SUM(connections) FROM daily WHERE day='2026-12-31'"
        ),
        1
    );
    assert_eq!(
        scalar(
            &db,
            "SELECT SUM(connections) FROM daily WHERE day='2027-01-01'"
        ),
        1
    );
    assert_eq!(scalar(&db, "SELECT COUNT(*) FROM visits"), 3);
    assert_eq!(scalar(&db, "SELECT COUNT(*) FROM day_visits"), 2);
    assert_eq!(scalar(&db, "SELECT SUM(up) FROM daily"), 40);
}

#[test]
fn reopening_exposes_persisted_retention_before_any_new_submission() {
    // 重启后立即展示既有细节保留范围，不等下一次写入才揭示截断。
    let dir = Directory::new();
    let writer = TrafficWriter::open(&dir.0, chrono_tz::UTC).unwrap();
    writer.submit(interval("record", NOW, 10)).unwrap();
    writer.flush().unwrap();
    let cutoff = writer.status().detail_cutoff_ms;
    assert!(cutoff.is_some());
    writer.close().unwrap();
    let reopened = TrafficWriter::open(&dir.0, chrono_tz::UTC).unwrap();
    assert_eq!(reopened.status().detail_cutoff_ms, cutoff);
    reopened.close().unwrap();
}

#[test]
fn file_lease_enforces_single_writer_and_releases_on_close() {
    // 同DB并行writer会混淆计数/保留范围：真实文件lease拒绝第二writer，关闭后释放。
    let dir = Directory::new();
    let first = TrafficWriter::open(&dir.0, chrono_tz::UTC).unwrap();
    assert!(TrafficWriter::open(&dir.0, chrono_tz::UTC).is_err());
    first.submit(interval("first", NOW, 10)).unwrap();
    first.close().unwrap();
    let second = TrafficWriter::open(&dir.0, chrono_tz::UTC).unwrap();
    second.submit(interval("second", NOW + 1000, 20)).unwrap();
    second.close().unwrap();
    assert_eq!(scalar(&dir.db(), "SELECT SUM(up) FROM daily"), 30);
}

#[test]
fn capacity_shortens_complete_old_buckets_and_reopens_explicit_range() {
    // 容量压缩必须能保留当前桶/日总量，并在重建后继续报告被缩短的范围。
    let dir = Directory::new();
    let writer = TrafficWriter::open(&dir.0, chrono_tz::UTC).unwrap();
    for i in 0..2000 {
        writer
            .submit(interval(
                &format!("capacity-old-{i}"),
                NOW + (i / 1000) * BUCKET_MS,
                1,
            ))
            .unwrap();
        if i % 128 == 127 {
            writer.flush().unwrap();
        }
    }
    writer.close().unwrap();
    let before = usage(&dir.0.join("traffic.sqlite3"))
        .unwrap()
        .database_bytes;
    let writer = TrafficWriter::open_with_capacity(&dir.0, chrono_tz::UTC, before * 3 / 4).unwrap();
    writer
        .submit(interval("capacity-new", NOW + 2 * BUCKET_MS, 7))
        .unwrap();
    writer.flush().unwrap();
    assert!(writer.status().capacity_shortened);
    assert!(writer.status().file_bytes <= before * 3 / 4);
    writer.close().unwrap();
    let writer = TrafficWriter::open(&dir.0, chrono_tz::UTC).unwrap();
    assert!(writer.status().capacity_shortened);
    writer.close().unwrap();
    assert_eq!(
        scalar(&dir.db(), "SELECT SUM(up) FROM daily WHERE kind='observed'"),
        2007
    );
}
