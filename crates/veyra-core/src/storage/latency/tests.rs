use super::*;
use crate::domain::PoolId;

// 保护重启后仍有最近十次实测，按测试时间排序；同名节点和新epoch不会混历史。
#[test]
fn disk_roundtrip_reopen_keeps_ten_actual_times_and_stable_ids() {
    let root = std::env::temp_dir().join(format!(
        "veyra-p208-history-{:?}",
        StateEpoch::fresh().unwrap()
    ));
    std::fs::create_dir(&root).unwrap();
    let path = root.join("latency.sqlite3");
    let epoch = StateEpoch::fresh().unwrap();
    let group = OutboundId::Pool(PoolId("group".into()));
    let node = NodeId("stable-node".into());
    let key = OutboundId::Node(node.clone());
    let mut store = LatencyStore::open(&path).unwrap();
    for time in (0..12).rev() {
        store
            .append(
                Some(&group),
                &LatencyRecord {
                    epoch: epoch.clone(),
                    node: node.clone(),
                    tested_at_ms: time,
                    instance_id: "owned".into(),
                    generation: 1,
                    outcome: LatencyOutcome::Success(50),
                },
            )
            .unwrap();
    }
    let expected = store.history(&epoch, &key).unwrap();
    assert_eq!(expected.len(), HISTORY_LIMIT);
    assert_eq!(
        expected.iter().map(|r| r.tested_at_ms).collect::<Vec<_>>(),
        (2..12).rev().collect::<Vec<_>>()
    );
    drop(store);
    let store = LatencyStore::open(&path).unwrap();
    assert_eq!(store.history(&epoch, &key).unwrap(), expected);
    assert_eq!(store.history(&epoch, &group).unwrap(), expected);
    assert!(
        store
            .history(&StateEpoch::fresh().unwrap(), &key)
            .unwrap()
            .is_empty()
    );
    assert!(
        store
            .history(
                &epoch,
                &OutboundId::Node(NodeId("other-same-display-name".into()))
            )
            .unwrap()
            .is_empty()
    );
    drop(store);
    std::fs::remove_dir_all(root).unwrap();
}

// 保护组历史写失败时节点历史和组历史均保旧；真实SQLite事务回滚而非Mock失败。
#[test]
fn failed_group_append_rolls_back_node_and_preserves_reopen_history() {
    let root = std::env::temp_dir().join(format!(
        "veyra-p208-rollback-{:?}",
        StateEpoch::fresh().unwrap()
    ));
    std::fs::create_dir(&root).unwrap();
    let path = root.join("latency.sqlite3");
    let epoch = StateEpoch::fresh().unwrap();
    let key = OutboundId::Node(NodeId("node".into()));
    let group = OutboundId::Pool(PoolId("group".into()));
    let mut record = LatencyRecord {
        epoch: epoch.clone(),
        node: NodeId("node".into()),
        tested_at_ms: 1,
        instance_id: "owned".into(),
        generation: 1,
        outcome: LatencyOutcome::Zero,
    };
    let mut store = LatencyStore::open(&path).unwrap();
    store.append(Some(&group), &record).unwrap();
    store.connection.execute_batch("CREATE TRIGGER reject_group BEFORE INSERT ON latency_history WHEN NEW.scope LIKE '%pool%' BEGIN SELECT RAISE(ABORT,'controlled write failure'); END;").unwrap();
    record.tested_at_ms = 2;
    record.outcome = LatencyOutcome::Timeout;
    assert!(store.append(Some(&group), &record).is_err());
    assert_eq!(store.history(&epoch, &key).unwrap().len(), 1);
    drop(store);
    let store = LatencyStore::open(&path).unwrap();
    assert_eq!(
        store.history(&epoch, &key).unwrap()[0].outcome,
        LatencyOutcome::Zero
    );
    assert_eq!(store.history(&epoch, &group).unwrap()[0].tested_at_ms, 1);
    drop(store);
    std::fs::remove_dir_all(root).unwrap();
}
