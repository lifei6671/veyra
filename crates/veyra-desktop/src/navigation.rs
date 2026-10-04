//! Identity and order follow OpenBoxApp.tsx and SettingsPage.tsx.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum Route {
    #[default]
    Overview,
    Proxies,
    Connections,
    Logs,
    Rules,
    Settings,
}
impl Route {
    pub const ALL: [Self; 6] = [
        Self::Overview,
        Self::Proxies,
        Self::Connections,
        Self::Logs,
        Self::Rules,
        Self::Settings,
    ];
    pub fn id(self) -> &'static str {
        match self {
            Self::Overview => "overview",
            Self::Proxies => "proxies",
            Self::Connections => "connections",
            Self::Logs => "logs",
            Self::Rules => "rules",
            Self::Settings => "settings",
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            Self::Overview => "概览",
            Self::Proxies => "代理",
            Self::Connections => "连接",
            Self::Logs => "日志",
            Self::Rules => "规则",
            Self::Settings => "设置",
        }
    }
    pub fn task(self) -> &'static str {
        match self {
            Self::Overview => "P3-05 / P3-06",
            Self::Proxies => "P2-08 / P3-07",
            Self::Connections => "P3-02",
            Self::Logs => "P3-03",
            Self::Rules => "P3-07 / P4-05 / P5-03",
            Self::Settings => "P1-04 / P2 / P4 / P5",
        }
    }
}
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum Category {
    #[default]
    Panel,
    Subscriptions,
    Groups,
    Routing,
    Clients,
    Chain,
    Share,
    Dns,
    Backend,
}
impl Category {
    pub const ALL: [Self; 9] = [
        Self::Panel,
        Self::Subscriptions,
        Self::Groups,
        Self::Routing,
        Self::Clients,
        Self::Chain,
        Self::Share,
        Self::Dns,
        Self::Backend,
    ];
    pub fn id(self) -> &'static str {
        match self {
            Self::Panel => "panel",
            Self::Subscriptions => "subscriptions",
            Self::Groups => "groups",
            Self::Routing => "routing",
            Self::Clients => "clients",
            Self::Chain => "chain",
            Self::Share => "share",
            Self::Dns => "dns",
            Self::Backend => "backend",
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            Self::Panel => "面板设置",
            Self::Subscriptions => "订阅管理",
            Self::Groups => "出站节点",
            Self::Routing => "目标分流",
            Self::Clients => "终端分流",
            Self::Chain => "链式代理",
            Self::Share => "共享网络",
            Self::Dns => "DNS 设置",
            Self::Backend => "后端设置",
        }
    }
    pub fn task(self) -> &'static str {
        match self {
            Self::Panel => "P1-04A / P1-04B",
            Self::Subscriptions => "P2-01 / P4-01",
            Self::Groups => "P4-02 / P4-03",
            Self::Routing => "P4-04 / P4-05A",
            Self::Clients => "P4-05B",
            Self::Chain => "P4-06",
            Self::Share => "P5-04 / P5-05",
            Self::Dns => "P5-01 / P5-02 / P5-03",
            Self::Backend => "P2-03 / P6 / P7",
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    // Protect stable source navigation identities, order and translated labels.
    #[test]
    fn six_routes_match_openbox() {
        assert_eq!(
            Route::ALL.map(Route::id),
            [
                "overview",
                "proxies",
                "connections",
                "logs",
                "rules",
                "settings"
            ]
        );
    }
    #[test]
    fn nine_categories_match_openbox() {
        assert_eq!(
            Category::ALL.map(Category::id),
            [
                "panel",
                "subscriptions",
                "groups",
                "routing",
                "clients",
                "chain",
                "share",
                "dns",
                "backend"
            ]
        );
        assert_eq!(
            Category::ALL.map(Category::label),
            [
                "面板设置",
                "订阅管理",
                "出站节点",
                "目标分流",
                "终端分流",
                "链式代理",
                "共享网络",
                "DNS 设置",
                "后端设置"
            ]
        );
    }
}
