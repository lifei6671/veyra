use super::{AppError, FieldPath, RouteTarget};
use serde::{Deserialize, Deserializer, Serialize};

/// Supported foundation fields only; this is not an OpenBox backup importer or full P4/P5 model.
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Profile {
    pub ipv6: bool,
    pub dns: ProfileDns,
    pub routing: ProfileRouting,
    pub tun: ProfileTun,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProfileDns {
    pub split: bool,
    pub direct: String,
    pub direct_port: u16,
    pub direct_extras: Vec<DnsUpstream>,
    pub proxy: String,
    pub proxy_port: u16,
    pub proxy_extras: Vec<DnsUpstream>,
}
impl Default for ProfileDns {
    fn default() -> Self {
        Self {
            split: false,
            direct: "1.1.1.1".into(),
            direct_port: 53,
            direct_extras: vec![],
            proxy: "1.1.1.1".into(),
            proxy_port: 53,
            proxy_extras: vec![],
        }
    }
}
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct DnsUpstream {
    pub server: String,
    pub protocol: DnsTransport,
    pub port: u16,
}
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum DnsTransport {
    Udp,
    Tcp,
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ProfileRouting {
    pub custom: Option<CustomRouting>,
}
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CustomRouting {
    pub name: Option<String>,
    pub enabled: bool,
    pub rules: Vec<CustomRoutingRule>,
}
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CustomRoutingRule {
    pub matcher: DomainMatcher,
    pub value: String,
    pub target: RouteTarget,
}
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum DomainMatcher {
    Domain,
    DomainSuffix,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProfileTun {
    pub auto_redirect: bool,
    pub stack: TunStack,
    pub mtu: u16,
    pub tcp_mss: u16,
}
impl Default for ProfileTun {
    fn default() -> Self {
        Self {
            auto_redirect: false,
            stack: TunStack::System,
            mtu: 1500,
            tcp_mss: 0,
        }
    }
}
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum TunStack {
    System,
    Gvisor,
    Mixed,
}

/// Missing is supplied by the field default; a present JSON null is always Null.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub enum Patch<T> {
    #[default]
    Missing,
    Null,
    Value(T),
}
impl<'de, T: Deserialize<'de>> Deserialize<'de> for Patch<T> {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        Ok(match Option::<T>::deserialize(deserializer)? {
            Some(value) => Self::Value(value),
            None => Self::Null,
        })
    }
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq)]
#[serde(default, rename_all = "camelCase", deny_unknown_fields)]
pub struct ProfilePatch {
    pub ipv6: Patch<bool>,
    pub dns: Patch<DnsPatch>,
    pub routing: Patch<RoutingPatch>,
    pub tun: Patch<TunPatch>,
}
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq)]
#[serde(default, rename_all = "camelCase", deny_unknown_fields)]
pub struct DnsPatch {
    pub split: Patch<bool>,
    pub direct: Patch<String>,
    pub direct_port: Patch<u16>,
    pub direct_extras: Patch<Vec<DnsUpstream>>,
    pub proxy: Patch<String>,
    pub proxy_port: Patch<u16>,
    pub proxy_extras: Patch<Vec<DnsUpstream>>,
}
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq)]
#[serde(default, deny_unknown_fields)]
pub struct RoutingPatch {
    pub custom: Patch<CustomRoutingPatch>,
}
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq)]
#[serde(default, deny_unknown_fields)]
pub struct CustomRoutingPatch {
    pub name: Patch<String>,
    pub enabled: Patch<bool>,
    pub rules: Patch<Vec<CustomRoutingRule>>,
}
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq)]
#[serde(default, rename_all = "camelCase", deny_unknown_fields)]
pub struct TunPatch {
    pub auto_redirect: Patch<bool>,
    pub stack: Patch<TunStack>,
    pub mtu: Patch<u16>,
    pub tcp_mss: Patch<u16>,
}

fn set<T>(target: &mut T, patch: Patch<T>, field: FieldPath) -> Result<(), AppError> {
    match patch {
        Patch::Missing => {}
        Patch::Null => return Err(AppError::validation(field)),
        Patch::Value(value) => *target = value,
    }
    Ok(())
}

impl Profile {
    /// Merge on a private candidate so invalid nulls/values cannot partially mutate the profile.
    pub fn patched(&self, patch: ProfilePatch) -> Result<Self, AppError> {
        let mut next = self.clone();
        set(&mut next.ipv6, patch.ipv6, FieldPath::Ipv6)?;
        match patch.dns {
            Patch::Missing => {}
            Patch::Null => return Err(AppError::validation(FieldPath::Dns)),
            Patch::Value(p) => {
                set(&mut next.dns.split, p.split, FieldPath::DnsSplit)?;
                set(&mut next.dns.direct, p.direct, FieldPath::DnsDirect)?;
                set(
                    &mut next.dns.direct_port,
                    p.direct_port,
                    FieldPath::DnsDirectPort,
                )?;
                set(
                    &mut next.dns.direct_extras,
                    p.direct_extras,
                    FieldPath::DnsDirectExtras,
                )?;
                set(&mut next.dns.proxy, p.proxy, FieldPath::DnsProxy)?;
                set(
                    &mut next.dns.proxy_port,
                    p.proxy_port,
                    FieldPath::DnsProxyPort,
                )?;
                set(
                    &mut next.dns.proxy_extras,
                    p.proxy_extras,
                    FieldPath::DnsProxyExtras,
                )?;
            }
        }
        match patch.routing {
            Patch::Missing => {}
            Patch::Null => return Err(AppError::validation(FieldPath::Routing)),
            Patch::Value(p) => match p.custom {
                Patch::Missing => {}
                Patch::Null => next.routing.custom = None,
                Patch::Value(p) => {
                    let mut custom = next.routing.custom.take().unwrap_or_default();
                    match p.name {
                        Patch::Missing => {}
                        Patch::Null => custom.name = None,
                        Patch::Value(v) => custom.name = Some(v),
                    }
                    set(
                        &mut custom.enabled,
                        p.enabled,
                        FieldPath::RoutingCustomEnabled,
                    )?;
                    set(&mut custom.rules, p.rules, FieldPath::RoutingCustomRules)?;
                    next.routing.custom = Some(custom);
                }
            },
        }
        match patch.tun {
            Patch::Missing => {}
            Patch::Null => return Err(AppError::validation(FieldPath::Tun)),
            Patch::Value(p) => {
                set(
                    &mut next.tun.auto_redirect,
                    p.auto_redirect,
                    FieldPath::TunAutoRedirect,
                )?;
                set(&mut next.tun.stack, p.stack, FieldPath::TunStack)?;
                set(&mut next.tun.mtu, p.mtu, FieldPath::TunMtu)?;
                set(&mut next.tun.tcp_mss, p.tcp_mss, FieldPath::TunTcpMss)?;
            }
        }
        next.validate()?;
        Ok(next)
    }
    pub fn validate(&self) -> Result<(), AppError> {
        for (server, port, field) in [
            (&self.dns.direct, self.dns.direct_port, FieldPath::DnsDirect),
            (&self.dns.proxy, self.dns.proxy_port, FieldPath::DnsProxy),
        ] {
            if server.trim().is_empty() {
                return Err(AppError::validation(field));
            }
            if port == 0 {
                return Err(AppError::validation(if field == FieldPath::DnsDirect {
                    FieldPath::DnsDirectPort
                } else {
                    FieldPath::DnsProxyPort
                }));
            }
        }
        for (extras, field) in [
            (&self.dns.direct_extras, FieldPath::DnsDirectExtras),
            (&self.dns.proxy_extras, FieldPath::DnsProxyExtras),
        ] {
            if extras
                .iter()
                .any(|v| v.port == 0 || v.server.trim().is_empty())
            {
                return Err(AppError::validation(field));
            }
        }
        if self.tun.mtu < 576 {
            return Err(AppError::validation(FieldPath::TunMtu));
        }
        if self.tun.tcp_mss != 0 && self.tun.tcp_mss >= self.tun.mtu {
            return Err(AppError::validation(FieldPath::TunTcpMss));
        }
        if self
            .routing
            .custom
            .as_ref()
            .is_some_and(|v| v.rules.iter().any(|r| r.value.trim().is_empty()))
        {
            return Err(AppError::validation(FieldPath::RoutingCustomRules));
        }
        Ok(())
    }
}
