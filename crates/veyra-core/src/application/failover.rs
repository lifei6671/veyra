//! 只计算主备选择建议。唯一 Runtime owner 消费建议后执行 pending/Controller/CAS；
//! 本模块不持有 Controller、不保存已确认选择，也不实现内核 URLTest。
use crate::application::runtime_snapshot::InstanceId;
use crate::domain::*;
use std::collections::HashSet;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum FailoverMode {
    Auto,
    ManualPin { lane_id: String },
}
/// 探测和选择建议使用同一完整身份；generation 是该组本次探测代次。
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FailoverProbeToken {
    pub instance: InstanceId,
    pub group: PoolId,
    pub selection: SelectionVersion,
    pub config: ConfigVersion,
    pub generation: u64,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum FailoverDecision {
    Keep,
    Select {
        token: FailoverProbeToken,
        lane_id: String,
    },
    AllFailed,
    PinnedUnavailable,
    IncompleteProbe,
    StaleProbe,
}

pub struct FailoverPolicy {
    token: FailoverProbeToken,
    lanes: Vec<String>,
    settings: FailoverSettings,
    mode: FailoverMode,
    failures: Vec<u32>,
    primary_since: Option<u64>,
    probe_pending: bool,
}
impl FailoverPolicy {
    pub fn new(
        instance: InstanceId,
        group: &NodeGroup,
        selection: SelectionVersion,
        config: ConfigVersion,
    ) -> Result<Self, GroupIssue> {
        if group.rule != GroupRule::Failover || group.lanes.is_empty() {
            return Err(GroupIssue::InvalidSettings(group.id.clone()));
        }
        let settings = group
            .failover
            .clone()
            .ok_or_else(|| GroupIssue::InvalidSettings(group.id.clone()))?;
        if settings.failure_threshold == 0
            || !(1000..=60000).contains(&settings.timeout_ms)
            || settings.recovery_hold_ms > 86400000
            || group.lanes.len() > 3
            || group
                .lanes
                .iter()
                .any(|lane| lane.id.trim().is_empty() || lane.members.is_empty())
            || group
                .lanes
                .iter()
                .map(|l| &l.id)
                .collect::<HashSet<_>>()
                .len()
                != group.lanes.len()
        {
            return Err(GroupIssue::InvalidSettings(group.id.clone()));
        }
        Ok(Self {
            token: FailoverProbeToken {
                instance,
                group: group.id.clone(),
                selection,
                config,
                generation: 0,
            },
            lanes: group.lanes.iter().map(|l| l.id.clone()).collect(),
            settings,
            mode: FailoverMode::Auto,
            failures: vec![0; group.lanes.len()],
            primary_since: None,
            probe_pending: false,
        })
    }
    /// 每次发起探测只允许消费一次完整结果；新批开始后前一批自动过期。
    pub fn begin_probe(&mut self) -> FailoverProbeToken {
        self.token.generation = self
            .token
            .generation
            .checked_add(1)
            .expect("probe generation exhausted");
        self.probe_pending = true;
        self.token.clone()
    }
    pub fn mode(&self) -> &FailoverMode {
        &self.mode
    }
    /// Runtime 在人工提交前调用，使所有旧探测失效。提交失败后的模式回退由 owner 决定。
    pub fn pin(
        &mut self,
        lane_id: &str,
        selection: SelectionVersion,
    ) -> Result<FailoverDecision, GroupIssue> {
        if !self.lanes.iter().any(|id| id == lane_id) {
            return Err(GroupIssue::InvalidLane {
                group: self.token.group.clone(),
                lane: lane_id.into(),
            });
        }
        self.invalidate(selection);
        self.mode = FailoverMode::ManualPin {
            lane_id: lane_id.into(),
        };
        Ok(FailoverDecision::Select {
            token: self.token.clone(),
            lane_id: lane_id.into(),
        })
    }
    /// 不沿用手动期间或旧 selection_revision 的失败累计；新 token 必须重新探测。
    pub fn resume_auto(&mut self, selection: SelectionVersion) {
        self.invalidate(selection);
        self.mode = FailoverMode::Auto;
    }
    /// owner 完成/清除 pending 后同步版本；旧批结果即刻失效。
    pub fn invalidate(&mut self, selection: SelectionVersion) {
        self.token.selection = selection;
        self.token.generation = self
            .token
            .generation
            .checked_add(1)
            .expect("probe generation exhausted");
        self.probe_pending = false;
        self.failures.fill(0);
        self.primary_since = None;
    }
    /// now_ms 来自 owner 的单调时钟，测试注入模拟值；current 只能是 Runtime 已确认线路。
    /// 一个批次须覆盖所有线路；网络超时由 owner 探测结果明确映射为 false。
    pub fn observe(
        &mut self,
        token: &FailoverProbeToken,
        current: Option<&str>,
        results: &[(String, bool)],
        now_ms: u64,
    ) -> FailoverDecision {
        if token != &self.token || !self.probe_pending {
            return FailoverDecision::StaleProbe;
        }
        if results.len() != self.lanes.len() {
            return FailoverDecision::IncompleteProbe;
        }
        let mut health = vec![];
        for lane in &self.lanes {
            let mut matching = results.iter().filter(|(id, _)| id == lane);
            let Some((_, available)) = matching.next() else {
                return FailoverDecision::IncompleteProbe;
            };
            if matching.next().is_some() {
                return FailoverDecision::IncompleteProbe;
            }
            health.push(*available);
        }
        self.probe_pending = false;
        if let FailoverMode::ManualPin { lane_id } = &self.mode {
            let index = self
                .lanes
                .iter()
                .position(|l| l == lane_id)
                .expect("validated pin");
            return if health[index] {
                FailoverDecision::Keep
            } else {
                FailoverDecision::PinnedUnavailable
            };
        }
        for (count, healthy) in self.failures.iter_mut().zip(&health) {
            *count = if *healthy { 0 } else { count.saturating_add(1) };
        }
        if health[0] {
            self.primary_since.get_or_insert(now_ms);
        } else {
            self.primary_since = None;
        }
        if !health.iter().any(|healthy| *healthy) {
            return FailoverDecision::AllFailed;
        }
        let current_index = current.and_then(|id| self.lanes.iter().position(|l| l == id));
        let target = match current_index {
            None => health.iter().position(|v| *v),
            Some(index)
                if index > 0
                    && self.settings.restore_primary
                    && health[0]
                    && self.primary_since.is_some_and(|since| {
                        now_ms.saturating_sub(since) >= self.settings.recovery_hold_ms
                    }) =>
            {
                Some(0)
            }
            Some(index)
                if !health[index] && self.failures[index] >= self.settings.failure_threshold =>
            {
                health
                    .iter()
                    .enumerate()
                    .skip(1)
                    .find(|(i, v)| *i != index && **v)
                    .map(|(i, _)| i)
            }
            _ => None,
        };
        if let Some(index) = target {
            FailoverDecision::Select {
                token: self.token.clone(),
                lane_id: self.lanes[index].clone(),
            }
        } else {
            FailoverDecision::Keep
        }
    }
}
