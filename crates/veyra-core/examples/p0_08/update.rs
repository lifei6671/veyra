//! GitHub Releases 最小原型；复用 P0-06 builder 的代理、DNS、TLS 与 timeout。
//! 元数据/下载共用一实例，不读取环境代理、不回退 Direct，不改 Runtime。
use reqwest::{Response, Url, dns::Resolve};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{io::Write, path::Path, sync::Arc, time::Duration};
use tokio::sync::watch;
use veyra_core::subscription::outbound::{RoutePolicy, client_builder};

pub const REPOSITORY: &str = "lifei6671/veyra";
pub const MANIFEST_NAME: &str = "veyra-release.json";
const LATEST: &str = "https://api.github.com/repos/lifei6671/veyra/releases/latest";
const MAX_METADATA: usize = 4 * 1024 * 1024;
const MAX_PACKAGE: u64 = 512 * 1024 * 1024;
const TOTAL: Duration = Duration::from_secs(300);
pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

/// 原型只消费稳定三段版本；预发行/不规范 tag 明确拒绝，不猜测渠道。
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct Version([u64; 3]);
impl Version {
    pub fn parse(text: &str) -> Result<Self> {
        let parts = text.split('.').collect::<Vec<_>>();
        if parts.len() != 3
            || parts.iter().any(|p| {
                p.is_empty()
                    || !p.bytes().all(|b| b.is_ascii_digit())
                    || (p.len() > 1 && p.starts_with('0'))
            })
        {
            return Err("仅支持稳定 X.Y.Z 版本".into());
        }
        Ok(Self([
            parts[0].parse()?,
            parts[1].parse()?,
            parts[2].parse()?,
        ]))
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Asset {
    pub name: String,
    pub size: u64,
    pub browser_download_url: String,
}
#[derive(Debug, Deserialize, Serialize)]
pub struct Release {
    pub tag_name: String,
    pub html_url: String,
    pub body: Option<String>,
    pub draft: bool,
    pub prerelease: bool,
    pub assets: Vec<Asset>,
}
impl Release {
    pub fn version(&self) -> Result<Version> {
        if self.draft || self.prerelease || !self.tag_name.starts_with('v') {
            return Err("拒绝草稿/预发行或非 vX.Y.Z tag".into());
        }
        let version = Version::parse(&self.tag_name[1..])?;
        if self.html_url
            != format!(
                "https://github.com/{REPOSITORY}/releases/tag/{}",
                self.tag_name
            )
        {
            return Err("发行仓库不匹配".into());
        }
        Ok(version)
    }
    pub fn asset(&self, name: &str) -> Result<&Asset> {
        self.version()?;
        let matches = self
            .assets
            .iter()
            .filter(|a| a.name == name)
            .collect::<Vec<_>>();
        if matches.len() != 1 {
            return Err("固定发行资源缺失或重复".into());
        }
        let asset = matches[0];
        if asset.browser_download_url != asset_url(&self.tag_name[1..], name) {
            return Err("固定资源下载来源不匹配".into());
        }
        Ok(asset)
    }
}
pub fn asset_url(version: &str, name: &str) -> String {
    format!("https://github.com/{REPOSITORY}/releases/download/v{version}/{name}")
}

/// 发布清单是完整性事实，不是发布者身份/代码签名；不包含用户配置。
#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Manifest {
    pub schema: u32,
    pub repository: String,
    pub app_version: String,
    pub helper_version: String,
    pub sing_box_version: String,
    pub resources_version: String,
    pub platform: String,
    pub minimum_system_version: String,
    pub asset_name: String,
    pub size: u64,
    pub sha256: String,
    pub source_commit: String,
    pub cargo_lock_sha256: String,
}
fn hex_digest(value: &str, length: usize) -> bool {
    value.len() == length
        && value
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}
impl Manifest {
    pub fn version(&self) -> Result<Version> {
        Version::parse(&self.app_version)
    }
    pub fn validate(&self, release: &Release) -> Result<()> {
        if self.schema != 1
            || self.repository != REPOSITORY
            || self.platform != "macos-arm64"
            || self.version()? != release.version()?
            || self.resources_version != self.app_version
            || self.sing_box_version != "1.14.0"
            || self.minimum_system_version != "26.0"
        {
            return Err("发布版本/平台/组件绑定不匹配".into());
        }
        Version::parse(&self.helper_version)?;
        if self.asset_name != format!("VeyraPrototype-{}-macos-arm64.zip", self.app_version)
            || self.size == 0
            || self.size > MAX_PACKAGE
            || !hex_digest(&self.sha256, 64)
            || !hex_digest(&self.source_commit, 40)
            || !hex_digest(&self.cargo_lock_sha256, 64)
        {
            return Err("资源名称/大小/摘要/源码身份非法".into());
        }
        if release.asset(&self.asset_name)?.size != self.size {
            return Err("GitHub 资源大小与清单不匹配".into());
        }
        Ok(())
    }
}

pub struct UpdateClient {
    client: reqwest::Client,
    // 仅测试代码可设置 loopback 源；CLI 不接受自定义发行仓库或下载 URL。
    #[cfg(test)]
    fixture: Option<Url>,
}
impl UpdateClient {
    pub fn new<R: Resolve + 'static>(policy: &RoutePolicy, resolver: Arc<R>) -> Result<Self> {
        let client = client_builder(policy, resolver, TOTAL)
            .map_err(|e| format!("显式出站客户端失败：{e:?}"))?
            .user_agent("Veyra-P0-08/0.1.0")
            .build()?;
        Ok(Self {
            client,
            #[cfg(test)]
            fixture: None,
        })
    }
    // 固定 GitHub API/仓库资源之外，仅允许 GitHub 的实际资源 CDN 重定向。
    // 所有跳转仍走同一显式客户端；没有订阅凭据或 Authorization 头。
    async fn response(&self, source: &str) -> Result<Response> {
        let mut url = Url::parse(source)?;
        for hop in 0..=5 {
            #[cfg(test)]
            let is_fixture = self
                .fixture
                .as_ref()
                .is_some_and(|base| url.origin() == base.origin());
            #[cfg(not(test))]
            let is_fixture = false;
            if !is_fixture
                && (url.scheme() != "https"
                    || !url.username().is_empty()
                    || url.password().is_some()
                    || url.port().is_some_and(|p| p != 443)
                    || !matches!(
                        url.host_str(),
                        Some(
                            "api.github.com"
                                | "github.com"
                                | "release-assets.githubusercontent.com"
                                | "objects.githubusercontent.com"
                        )
                    ))
            {
                return Err("拒绝非 GitHub HTTPS 重定向".into());
            }
            let response = self.client.get(url.clone()).send().await?;
            if response.status().is_redirection() {
                if hop == 5 {
                    return Err("重定向超过五次".into());
                }
                let location = response
                    .headers()
                    .get(reqwest::header::LOCATION)
                    .ok_or("重定向缺少 Location")?
                    .to_str()?;
                url = url.join(location)?;
            } else if response.status() == reqwest::StatusCode::OK {
                return Ok(response);
            } else {
                return Err(format!("HTTP 状态 {}", response.status()).into());
            }
        }
        unreachable!("循环的最后一次重定向明确返回错误")
    }
    fn source(&self, source: &str) -> String {
        #[cfg(test)]
        if let Some(base) = &self.fixture {
            return base
                .join(Url::parse(source).expect("固定URL").path())
                .expect("fixture路径")
                .to_string();
        }
        source.to_owned()
    }
    async fn metadata<T: serde::de::DeserializeOwned>(&self, source: &str) -> Result<T> {
        let mut response = self.response(&self.source(source)).await?;
        let mut bytes = Vec::new();
        while let Some(chunk) = response.chunk().await? {
            if bytes.len().saturating_add(chunk.len()) > MAX_METADATA {
                return Err("发行元数据超过4MiB".into());
            }
            bytes.extend_from_slice(&chunk);
        }
        Ok(serde_json::from_slice(&bytes)?)
    }
    pub async fn check(&self, cancel: &mut watch::Receiver<bool>) -> Result<(Release, Manifest)> {
        bounded(cancel, async {
            let release: Release = self.metadata(LATEST).await?;
            let manifest_asset = release.asset(MANIFEST_NAME)?;
            if manifest_asset.size == 0 || manifest_asset.size > MAX_METADATA as u64 {
                return Err("发行清单大小非法".into());
            }
            let manifest: Manifest = self.metadata(&manifest_asset.browser_download_url).await?;
            manifest.validate(&release)?;
            Ok((release, manifest))
        })
        .await
    }
    pub async fn download(
        &self,
        manifest: &Manifest,
        output: &Path,
        cancel: &mut watch::Receiver<bool>,
    ) -> Result<()> {
        // create_new 保护已有文件；只清理本次确实创建的 .part，不覆盖用户包。
        // 下载入口仅消费已经通过 check 的清单，CLI 不提供任意 URL 接口。
        if output.exists() {
            return Err("目标包已存在".into());
        }
        let part = output.with_extension("zip.part");
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&part)?;
        let result = bounded(cancel, async {
            let mut response = self
                .response(&self.source(&asset_url(&manifest.app_version, &manifest.asset_name)))
                .await?;
            if response
                .content_length()
                .is_some_and(|n| n != manifest.size)
            {
                return Err("响应大小与清单不匹配".into());
            }
            let mut count = 0_u64;
            let mut digest = Sha256::new();
            while let Some(chunk) = response.chunk().await? {
                count += chunk.len() as u64;
                if count > manifest.size {
                    return Err("包超过声明大小".into());
                }
                file.write_all(&chunk)?;
                digest.update(&chunk);
            }
            if count != manifest.size || format!("{:x}", digest.finalize()) != manifest.sha256 {
                return Err("包大小或 SHA256 不匹配".into());
            }
            file.sync_all()?;
            // 同目录硬链接以不覆盖方式交接；失败保留已有目的文件。
            std::fs::hard_link(&part, output)?;
            Ok(())
        })
        .await;
        drop(file);
        std::fs::remove_file(part)?;
        result
    }
}

async fn bounded<T>(
    cancel: &mut watch::Receiver<bool>,
    work: impl std::future::Future<Output = Result<T>>,
) -> Result<T> {
    if *cancel.borrow() {
        return Err("用户取消".into());
    }
    // Sender关闭表示CLI stdin EOF，不能误报用户取消；仍受总timeout限制。
    let cancelled = async {
        while cancel.changed().await.is_ok() {
            if *cancel.borrow() {
                return;
            }
        }
        std::future::pending::<()>().await;
    };
    tokio::select! {
        biased;
        _ = cancelled => Err("用户取消".into()),
        result = tokio::time::timeout(TOTAL, work) => result.map_err(|_| "总操作超时")?,
    }
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
