//! P0-06只发自身请求的探针；不读订阅、系统代理或Runtime配置，不启动TUN。
use reqwest::header::HeaderMap;
use std::{sync::Arc, time::Duration};
use veyra_core::subscription::{
    FetchResult,
    outbound::{BootstrapResolver, RoutePolicy, RunningProxy, client_builder, fetch_on_client},
};

fn main() {
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .expect("probe runtime");
    runtime.block_on(async {
        let source=args.get(1).expect("usage: direct URL | via URL INSTANCE_ID LOOPBACK_ADDRESS");
        let policy=match args.first().map(String::as_str) {
            Some("direct")=>RoutePolicy::Direct,
            Some("via")=>RoutePolicy::ViaRunningProxy(RunningProxy::from_current(args.get(2).zip(args.get(3)).map(|(id,address)|(id.clone(),address.parse().expect("loopback socket address")))).expect("current proxy required")),
            _=>panic!("explicit direct or via required"),
        };
        let timeout=Duration::from_secs(30);
        let resolver=Arc::new(BootstrapResolver::new(Duration::from_secs(5)).expect("bootstrap TLS client"));
        let client=client_builder(&policy,resolver.clone(),timeout).expect("route builder").build().expect("client");
        let result=fetch_on_client(&client,source,HeaderMap::new(),timeout).await;
        let (outcome,body,hops,error)=match result {
            Ok(response)=>{
                let body=match response.result{FetchResult::Modified{body,..}=>Some(body),FetchResult::NotModified{..}=>None};
                // HTTP成功不冒充物理直连或Veyra受管TUN验收。
                ("HTTP_SUCCESS",body,response.hops,None)
            },
            Err(error)=>("FAIL",None,Vec::new(),Some(format!("{error:?}"))),
        };
        let proxy=match &policy{RoutePolicy::Direct=>None,RoutePolicy::ViaRunningProxy(p)=>Some(serde_json::json!({"instance_id":p.instance_id(),"address":p.address()}))};
        println!("{}",serde_json::json!({"event":"p0_06_outbound","policy":if proxy.is_none(){"Direct"}else{"ViaRunningProxy"},"explicit_proxy":proxy,"endpoint":source,"TLS_enabled":source.starts_with("https://"),"outcome":outcome,"body":body,"hops":hops,"dns":resolver.observations(),"error":error,"silent_fallback":false,"subscription_dependencies":0}));
        if outcome=="FAIL" {std::process::exit(1);}
    });
}
