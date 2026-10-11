//! Runtime Ready 能力的只读消费面；不读取 Saved state、不提供任何选择写操作。
use std::collections::BTreeMap;

use serde::Serialize;

use crate::{
    application::runtime_snapshot::InstanceId,
    domain::{ConfigVersion, OutboundId, PoolId},
    singbox::clash_api::ClashApiClient,
    subscription::outbound::RunningProxy,
};

/// 完整身份随数据传递；这是已应用计划的直接成员，不是当前出口或递归叶子展开。
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct AppliedProxyFacts {
    pub instance_id: InstanceId,
    pub config: ConfigVersion,
    pub pools: BTreeMap<PoolId, Vec<OutboundId>>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct AppliedPoolObservation {
    pub instance_id: InstanceId,
    pub config: ConfigVersion,
    pub pool_id: PoolId,
    /// 仅一次鉴权 GET 的 now 且必须属于已应用成员。None = Unknown，不回退默认/Saved。
    /// Pool 表示仍是组引用，不声称它是最终叶子出口。
    pub current: Option<OutboundId>,
}

/// Controller 的实际 UDP 能力；缺失/非法/请求失败保留 Unknown，不根据协议猜测。
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct AppliedProxyMetadata {
    pub instance_id: InstanceId,
    pub config: ConfigVersion,
    pub outbound_id: OutboundId,
    pub udp: Option<bool>,
}

impl RunningProxy {
    /// 只认发布此能力的 Runtime 实例及已应用配置。旧恢复计划没有完整索引时不可用。
    pub fn applied_proxy_facts(
        &self,
        instance: &InstanceId,
        config: &ConfigVersion,
    ) -> Option<AppliedProxyFacts> {
        let index = self.applied_index.as_ref()?;
        if !self.is_valid() || self.instance_id() != instance.0 || &index.config != config {
            return None;
        }
        Some(AppliedProxyFacts {
            instance_id: instance.clone(),
            config: config.clone(),
            pools: index.applied_members.as_ref()?.clone(),
        })
    }

    /// 仅接受当前已应用索引中的出口，复用固定鉴权 GET 和实例撤销取消。
    /// outer None 表示身份/出口不可用，Some(udp=None) 表示已应用但未读到有效布尔。
    pub async fn observe_applied_proxy_metadata(
        &self,
        instance: &InstanceId,
        config: &ConfigVersion,
        outbound: &OutboundId,
    ) -> Option<AppliedProxyMetadata> {
        self.applied_proxy_facts(instance, config)?;
        let index = self.applied_index.as_ref()?;
        let tag = match outbound {
            OutboundId::Node(id) => index.nodes.get(id)?.clone(),
            OutboundId::Pool(id) => index.applied_pool_tag(id)?,
            // 封闭 Compiler/恢复校验强制保留 direct；Block 只在实际已应用组引用时存在。
            OutboundId::Direct => "direct".into(),
            OutboundId::Block => {
                // Saved 的 Block 候选不构成应用证明；仅按当前实例的封闭编译索引查询。
                if !index
                    .applied_members
                    .as_ref()?
                    .values()
                    .any(|members| members.contains(&OutboundId::Block))
                {
                    return None;
                }
                "block".into()
            }
        };
        let udp = if let Some(endpoint) = &self.controller {
            if let Ok(client) = ClashApiClient::managed(endpoint) {
                self.while_valid(client.read_proxy_udp(&tag))
                    .await
                    .ok()?
                    .ok()
                    .flatten()
            } else {
                None
            }
        } else {
            None
        };
        // I/O 完成与映射之间仍可能撤销；旧实例的事实不得投影到新页面。
        self.applied_proxy_facts(instance, config)?;
        Some(AppliedProxyMetadata {
            instance_id: instance.clone(),
            config: config.clone(),
            outbound_id: outbound.clone(),
            udp,
        })
    }

    /// outer None = 能力/身份/组不可用；Some(current=None) = 组已应用，但实际出口未知。
    /// GET 复用既有固定端点、鉴权、两秒超时；实例撤销取消在途请求。
    pub async fn observe_applied_pool(
        &self,
        instance: &InstanceId,
        config: &ConfigVersion,
        pool: &PoolId,
    ) -> Option<AppliedPoolObservation> {
        self.applied_proxy_facts(instance, config)?;
        let index = self.applied_index.as_ref()?;
        let tag = index.applied_pool_tag(pool)?;
        let current = if let Some(endpoint) = &self.controller {
            let client = ClashApiClient::managed(endpoint).ok();
            if let Some(client) = client {
                let response = self.while_valid(client.read_selector(&tag)).await.ok()?;
                response
                    .ok()
                    .and_then(|now| index.applied_member(pool, &now))
            } else {
                None
            }
        } else {
            None
        };
        // 撤销可能发生在网络返回到映射之间；不把旧响应归给替换后的实例。
        self.applied_proxy_facts(instance, config)?;
        Some(AppliedPoolObservation {
            instance_id: instance.clone(),
            config: config.clone(),
            pool_id: pool.clone(),
            current,
        })
    }
}
