//! 分享只保存用户配置；Debug 不泄露访问凭据。
use serde::{Deserialize, Serialize};
use std::{collections::HashSet, net::SocketAddr};

#[derive(Clone, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SubscriptionShare {
    pub id: String,
    pub name: String,
    pub host: String,
    pub protocol: String,
    pub listen: SocketAddr,
    pub subscription_ids: Vec<super::SubscriptionId>,
    pub token: String,
    pub enabled: bool,
}
impl std::fmt::Debug for SubscriptionShare {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("SubscriptionShare([redacted])")
    }
}
impl SubscriptionShare {
    pub fn url(&self) -> String {
        format!("http://{}/sub/{}", self.host, self.token)
    }
    pub fn validate(&self) -> Result<(), &'static str> {
        if self.id.is_empty()
            || self.name.trim().is_empty()
            || self.name.len() > 256
            || self.host.len() > 260
            || self
                .host
                .chars()
                .any(|c| c.is_whitespace() || matches!(c, '/' | '\\'))
        {
            return Err("invalid name");
        }
        if self.protocol != "http" {
            return Err("HTTPS requires a certificate");
        }
        let url =
            reqwest::Url::parse(&format!("http://{}", self.host)).map_err(|_| "invalid host")?;
        if url.host_str().is_some_and(|h| {
            h.trim_matches(['[', ']'])
                .parse::<std::net::IpAddr>()
                .is_ok_and(|ip| ip.is_unspecified())
        }) {
            return Err("advertised address must not be unspecified");
        }
        if url.host_str().is_none()
            || !url.username().is_empty()
            || url.password().is_some()
            || url.path() != "/"
            || url.query().is_some()
            || url.fragment().is_some()
            || url.port_or_known_default() != Some(self.listen.port())
            || self.listen.port() == 0
        {
            return Err("advertised port must match listener");
        }
        if self.token.len() != 64 || !self.token.bytes().all(|b| b.is_ascii_hexdigit()) {
            return Err("invalid token");
        }
        if self.subscription_ids.is_empty()
            || self.subscription_ids.iter().collect::<HashSet<_>>().len()
                != self.subscription_ids.len()
        {
            return Err("invalid subscription selection");
        }
        Ok(())
    }
}
pub fn validate_shares(shares: &[SubscriptionShare]) -> Result<(), &'static str> {
    let mut ids = HashSet::new();
    let mut tokens = HashSet::new();
    for share in shares {
        share.validate()?;
        if !ids.insert(&share.id) || !tokens.insert(&share.token) {
            return Err("duplicate share");
        }
    }
    Ok(())
}
