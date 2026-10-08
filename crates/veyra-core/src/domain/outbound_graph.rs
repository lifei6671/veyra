use std::collections::{BTreeMap, BTreeSet};

/// 同一引用图契约供 Base 及后续 Group/Chain 注册使用，不依赖内核标签或业务种类。
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum OutboundGraphError<Id> {
    Duplicate(Id),
    Dangling { source: Id, target: Id },
    SelfReference(Id),
    Cycle(Vec<Id>),
}

/// 注册方只提供稳定身份与直接引用；统一检查重复、自引用、悬空及多跳循环。
pub fn validate_outbound_graph<Id: Clone + Ord>(
    entries: impl IntoIterator<Item = (Id, Vec<Id>)>,
) -> Result<(), OutboundGraphError<Id>> {
    let mut graph = BTreeMap::new();
    for (id, references) in entries {
        if graph.insert(id.clone(), references).is_some() {
            return Err(OutboundGraphError::Duplicate(id));
        }
    }
    for (source, references) in &graph {
        for target in references {
            if source == target {
                return Err(OutboundGraphError::SelfReference(source.clone()));
            }
            if !graph.contains_key(target) {
                return Err(OutboundGraphError::Dangling {
                    source: source.clone(),
                    target: target.clone(),
                });
            }
        }
    }
    // 显式 DFS 栈避免合法长链占用调用栈；已完成节点只访问一次。
    let mut done = BTreeSet::new();
    let mut active = BTreeSet::new();
    for root in graph.keys() {
        if done.contains(root) {
            continue;
        }
        let mut stack = vec![(root.clone(), 0)];
        active.insert(root.clone());
        while let Some((id, index)) = stack.last_mut() {
            if let Some(target) = graph[id].get(*index) {
                *index += 1;
                if active.contains(target) {
                    let start = stack.iter().position(|(id, _)| id == target).unwrap();
                    let mut path = stack[start..]
                        .iter()
                        .map(|(id, _)| id.clone())
                        .collect::<Vec<_>>();
                    path.push(target.clone());
                    return Err(OutboundGraphError::Cycle(path));
                }
                if !done.contains(target) {
                    active.insert(target.clone());
                    stack.push((target.clone(), 0));
                }
            } else {
                let (id, _) = stack.pop().unwrap();
                active.remove(&id);
                done.insert(id);
            }
        }
    }
    Ok(())
}
