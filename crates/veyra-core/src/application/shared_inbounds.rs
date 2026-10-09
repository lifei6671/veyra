//! 持久化命令与端口提示；此模块不拥有 sing-box 进程或恢复状态。
use crate::domain::{SharedServer, SharedTransport, shared_sockets_overlap};
use std::net::{SocketAddr, TcpListener, UdpSocket};

#[derive(Clone, Debug)]
pub enum SharedServerCommand {
    Create(SharedServer),
    Update(SharedServer),
    SetEnabled {
        id: String,
        enabled: bool,
    },
    Delete {
        id: String,
    },
    /// 完整 ID 排序，拒绝遗漏、重复或外来 ID。
    Reorder(Vec<String>),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SharedPortIssue {
    Reserved,
    Server,
    Listening,
    Unavailable,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SharedPortWarning {
    pub transport: SharedTransport,
    pub issue: SharedPortIssue,
}
#[derive(Clone, Copy, Debug)]
pub struct SharedPortReservation {
    pub address: SocketAddr,
    pub transport: SharedTransport,
}
/// owner 显式传入保留端口；检查结束立即释放 socket，结果只是提示，不证明最终监听成功。
/// exclude_id 只用于编辑既有配置；正在运行的自有 listener 同样可能报告 Listening。
pub fn preflight_shared_port(
    server: &SharedServer,
    servers: &[SharedServer],
    reserved: &[SharedPortReservation],
    exclude_id: Option<&str>,
) -> Vec<SharedPortWarning> {
    server
        .transports()
        .iter()
        .filter_map(|&transport| {
            let address = server.socket_addr();
            let issue = if reserved
                .iter()
                .any(|r| r.transport == transport && shared_sockets_overlap(address, r.address))
            {
                Some(SharedPortIssue::Reserved)
            } else if servers.iter().any(|s| {
                s.enabled
                    && Some(s.id.as_str()) != exclude_id
                    && s.transports().contains(&transport)
                    && shared_sockets_overlap(address, s.socket_addr())
            }) {
                Some(SharedPortIssue::Server)
            } else {
                let result = match transport {
                    SharedTransport::Tcp => TcpListener::bind(address).map(drop),
                    SharedTransport::Udp => UdpSocket::bind(address).map(drop),
                };
                result.err().map(|e| {
                    if e.kind() == std::io::ErrorKind::AddrInUse {
                        SharedPortIssue::Listening
                    } else {
                        SharedPortIssue::Unavailable
                    }
                })
            };
            issue.map(|issue| SharedPortWarning { transport, issue })
        })
        .collect()
}

#[cfg(test)]
#[path = "shared_inbounds/tests.rs"]
mod tests;

pub mod sharing;
