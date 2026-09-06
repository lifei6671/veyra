async (page) => {
  const checks = [];
  const errors = [];
  const check = (value, name) => { if (!value) throw new Error(name); checks.push(name); };
  page.on("pageerror", (error) => errors.push(error.message));
  await page.unroute("**/src/main.tsx*");
  await page.route("**/src/main.tsx*", async (route) => {
    const response = await route.fetch();
    const setup = `import { mockIPC } from '/node_modules/@tauri-apps/api/mocks.js'; import { emit } from '/node_modules/@tauri-apps/api/event.js';
const emptyFilter=()=>({regions:[],protocols:[],includeKeywords:[],excludeKeywords:[],includeNodeIds:[],excludeNodeIds:[]});
const pool={id:'pool-a',name:'手动组',kind:'custom',enabled:true,sources:[{providerId:'provider-a',filter:emptyFilter()}],selection:{type:'manual',selectedNodeId:'node-a'},resolvedNodeIds:['node-a','node-b']};
const routeA={id:'route-a',name:'规则 A',enabled:true,priority:0,matcher:{type:'domain',values:['a.test']},target:{type:'direct'}};
const routeB={id:'route-b',name:'规则 B',enabled:false,priority:1,matcher:{type:'domainSuffix',values:['b.test']},target:{type:'block'}};
window.fixture012={calls:[],queryMode:'busy',nextId:1,selectorRelease:null,applyQueries:0,emit,snapshot:{revision:7,desiredGeneration:4,appliedGeneration:3,runtimeState:'ready',applyState:{type:'savedPendingApply'},activeSubscriptionId:'sub-a',appliedSubscriptionId:'sub-a',providers:[{id:'provider-a',subscriptionId:'sub-a',name:'订阅 A'}],nodes:[{id:'node-a',providerId:'provider-a',name:'香港 A',protocol:'socks'},{id:'node-b',providerId:'provider-a',name:'日本 B',protocol:'socks'}],defaultTarget:{type:'direct'},pools:[pool],routes:[routeA,routeB],selectors:[{poolId:'pool-a',desiredNodeId:'node-a',runtimeNodeId:'node-a',state:'inSync'}]}};
const clone=()=>structuredClone(window.fixture012.snapshot);
const saved=(mutation)=>{const f=window.fixture012;f.snapshot.revision++;if(mutation.type!=='setManualSelection')f.snapshot.desiredGeneration++;f.snapshot.applyState={type:'savedPendingApply'};return{status:'ok',outcome:{type:'saved'},snapshot:clone()}};
mockIPC(async(command,args)=>{const f=window.fixture012;f.calls.push({command,args:structuredClone(args??{})});
 if(command==='bootstrap_status')return{application:'Veyra',status:'ready'};
 if(command==='runtime_observation_snapshot')return{source:'runtime',revision:1,observedAtMs:1,trafficHistory:[],captureMode:'off',sidecarLifecycle:'ready',uploadRateBps:0,downloadRateBps:0,uploadTotalBytes:0,downloadTotalBytes:0,connectionCount:0,logSummary:[],appliedSubscriptionId:'sub-a',appliedConfigurationGeneration:3,subscriptionSwitch:null,managedProxyAvailable:false,coreMemoryBytes:null};
 if(command==='list_subscriptions')return{status:'ok',subscriptions:[]};
 if(command==='get_proxy_routing_snapshot'){
   if(f.queryMode==='busy')return{status:'error',error:'busy'};
   if(f.snapshot.applyState.type==='applying'&&++f.applyQueries>=1){f.snapshot.appliedGeneration=f.snapshot.desiredGeneration;f.snapshot.runtimeState='ready';f.snapshot.applyState={type:'applied'};}
   return{status:'ok',snapshot:clone()};
 }
 if(command==='mutate_proxy_routing'){
   const m=args.request.mutation;
   if(m.type==='createCustomPool'){
     if(m.name==='保留输入'){return{status:'error',error:'validationFailed',revision:f.snapshot.revision};}
     const id='pool-'+(++f.nextId);f.snapshot.pools.push({id,name:m.name,kind:'custom',enabled:m.enabled,sources:m.sources,selection:m.selection,resolvedNodeIds:['node-a','node-b']});f.snapshot.selectors.push({poolId:id,desiredNodeId:m.selection.type==='manual'?m.selection.selectedNodeId:null,runtimeNodeId:null,state:'savedOnly'});return saved(m);
   }
   if(m.type==='updateCustomPool'){const i=f.snapshot.pools.findIndex(x=>x.id===m.id);f.snapshot.pools[i]={...f.snapshot.pools[i],name:m.name,enabled:m.enabled,sources:m.sources,selection:m.selection};return saved(m);}
   if(m.type==='deleteCustomPool'){f.snapshot.pools=f.snapshot.pools.filter(x=>x.id!==m.id);f.snapshot.selectors=f.snapshot.selectors.filter(x=>x.poolId!==m.id);return saved(m);}
   if(m.type==='setManualSelection')return await new Promise(resolve=>{f.selectorRelease=()=>{const p=f.snapshot.pools.find(x=>x.id===m.poolId);p.selection={type:'manual',selectedNodeId:m.nodeId};const s=f.snapshot.selectors.find(x=>x.poolId===m.poolId);s.desiredNodeId=m.nodeId;s.runtimeNodeId=m.nodeId;s.state='inSync';f.snapshot.revision++;resolve({status:'ok',outcome:{type:'selectorApplied',poolId:m.poolId,nodeId:m.nodeId},snapshot:clone()});};});
   if(m.type==='setDefaultTarget'){f.snapshot.defaultTarget=m.target;return saved(m);}
   if(m.type==='createRoute'){const id='route-'+(++f.nextId);f.snapshot.routes.splice(m.insertAt,0,{id,name:m.name,enabled:m.enabled,priority:m.insertAt,matcher:m.matcher,target:m.target});f.snapshot.routes.forEach((x,i)=>x.priority=i);return saved(m);}
   if(m.type==='updateRoute'){const i=f.snapshot.routes.findIndex(x=>x.id===m.id);f.snapshot.routes[i]={...f.snapshot.routes[i],name:m.name,enabled:m.enabled,matcher:m.matcher,target:m.target};return saved(m);}
   if(m.type==='deleteRoute'){f.snapshot.routes=f.snapshot.routes.filter(x=>x.id!==m.id);f.snapshot.routes.forEach((x,i)=>x.priority=i);return saved(m);}
   if(m.type==='reorderRoutes'){f.snapshot.routes=m.routeIds.map((id,i)=>({...f.snapshot.routes.find(x=>x.id===id),priority:i}));return saved(m);}
   if(m.type==='applyConfiguration'){f.snapshot.runtimeState='transitioning';f.snapshot.applyState={type:'applying',operationId:'operation-browser'};f.applyQueries=0;return{status:'ok',outcome:{type:'applyStarted',operationId:'operation-browser'},snapshot:clone()};}
 }
 throw new Error('unexpected command '+command);
},{shouldMockEvents:true});`;
    await route.fulfill({ response, body: setup + await response.text() });
  });

  await page.setViewportSize({ width: 960, height: 640 });
  await page.goto("http://127.0.0.1:1421");
  const proxies = page.locator("#page-outbounds");
  const routing = page.locator("#page-routing");
  await page.getByRole("button", { name: "出口组", exact: true }).click();
  await proxies.getByText("配置正在使用中", { exact: true }).waitFor();
  check(await proxies.getByRole("button", { name: "重试", exact: true }).isVisible(), "query error exposes retry");
  await page.evaluate(() => { window.fixture012.queryMode = "ok"; });
  await proxies.getByRole("button", { name: "重试", exact: true }).click();
  await proxies.getByText("手动组", { exact: true }).waitFor();
  check(await proxies.getByText("有未应用更改", { exact: true }).isVisible(), "authoritative dirty state rendered");

  await proxies.getByRole("button", { name: "新建出口组", exact: true }).click();
  const poolDialog = proxies.getByRole("dialog", { name: "新建自定义出口组" });
  await poolDialog.waitFor();
  await poolDialog.getByLabel("名称", { exact: true }).fill("保留输入");
  await poolDialog.getByRole("checkbox", { name: "订阅 A", exact: true }).check();
  const initialNode = poolDialog.locator("label").filter({ hasText: "初始节点" }).locator("select");
  await initialNode.waitFor();
  await initialNode.selectOption("node-a");
  await poolDialog.getByRole("button", { name: "保存", exact: true }).click();
  await proxies.getByText("更改未通过完整配置校验", { exact: true }).waitFor();
  check(await poolDialog.getByLabel("名称", { exact: true }).inputValue() === "保留输入", "failed create preserves dialog input");
  await page.keyboard.press("Escape");
  check(await poolDialog.isHidden(), "Escape closes idle dialog");
  check(await proxies.getByRole("button", { name: "新建出口组", exact: true }).evaluate(e => e === document.activeElement), "dialog restores trigger focus");

  await proxies.getByRole("button", { name: "新建出口组", exact: true }).click();
  const urlTestDialog = proxies.getByRole("dialog", { name: "新建自定义出口组" });
  await urlTestDialog.getByLabel("名称", { exact: true }).fill("自动组");
  await urlTestDialog.getByRole("checkbox", { name: "订阅 A", exact: true }).check();
  await urlTestDialog.getByRole("radio", { name: "自动测试", exact: true }).check();
  await urlTestDialog.getByRole("button", { name: "保存", exact: true }).click();
  await proxies.getByText("自动组", { exact: true }).waitFor();
  check(await page.evaluate(() => window.fixture012.calls.some(x => x.command === 'mutate_proxy_routing' && x.args.request.mutation.type === 'createCustomPool' && x.args.request.mutation.selection.type === 'urlTest')), "UrlTest pool create mutation exact");
  await proxies.getByRole("button", { name: "自动组 操作", exact: true }).click();
  await proxies.getByRole("menuitem", { name: "编辑", exact: true }).click();
  const editPool = proxies.getByRole("dialog", { name: "编辑自定义出口组" });
  await editPool.getByLabel("名称", { exact: true }).fill("自动组改");
  await editPool.getByRole("button", { name: "保存", exact: true }).click();
  await proxies.getByText("自动组改", { exact: true }).waitFor();
  await proxies.getByRole("button", { name: "自动组改 操作", exact: true }).click();
  await proxies.getByRole("menuitem", { name: "删除", exact: true }).click();
  check(await proxies.getByText("自动组改", { exact: true }).count() === 0, "Pool edit and delete use authoritative snapshots");

  await proxies.getByRole("button", { name: /^手动组 自定义/ }).click();
  const nodeB = proxies.getByRole("button", { name: /日本 B/ });
  await nodeB.click();
  await proxies.getByText("切换中…", { exact: true }).waitFor();
  check(await nodeB.getAttribute("aria-busy") === "true", "selector target exposes aria-busy");
  check(await proxies.getByRole("button", { name: /香港 A/ }).getAttribute("aria-pressed") === "true", "pending does not patch authoritative selected member");
  await page.evaluate(() => window.fixture012.selectorRelease());
  await proxies.getByText("节点已切换并经运行时确认", { exact: true }).waitFor();
  check(await nodeB.getAttribute("aria-pressed") === "true", "selector terminal snapshot owns selected member");

  await proxies.getByRole("button", { name: "手动组 操作", exact: true }).click();
  const menu = proxies.getByRole("menu");
  await menu.waitFor();
  check(await proxies.getByRole("menuitem", { name: "编辑", exact: true }).evaluate(e => e === document.activeElement), "context menu focuses first action");
  await page.keyboard.press("ArrowDown");
  check(await proxies.getByRole("menuitem", { name: "删除", exact: true }).evaluate(e => e === document.activeElement), "context menu arrow navigation");
  await page.keyboard.press("Escape");
  check(await proxies.getByRole("button", { name: "手动组 操作", exact: true }).evaluate(e => e === document.activeElement), "context menu Escape restores trigger focus");

  await page.getByRole("button", { name: "分流", exact: true }).click();
  await routing.getByLabel("默认出口", { exact: true }).selectOption("block");
  await routing.getByText("更改已保存，等待应用", { exact: true }).waitFor();
  check(await page.evaluate(() => window.fixture012.calls.some(x => x.command === 'mutate_proxy_routing' && x.args.request.mutation.type === 'setDefaultTarget' && x.args.request.mutation.target.type === 'block')), "default target mutation exact");

  await routing.getByRole("button", { name: "新建规则", exact: true }).click();
  const routeDialog = routing.getByRole("dialog", { name: "新建分流规则" });
  await routeDialog.getByLabel("名称", { exact: true }).fill("规则 C");
  await routeDialog.getByLabel(/匹配值/).fill("c.test");
  await routeDialog.getByRole("button", { name: "保存", exact: true }).click();
  await routing.getByText("规则 C", { exact: true }).waitFor();
  await routing.getByRole("button", { name: "编辑 规则 C", exact: true }).click();
  const editRoute = routing.getByRole("dialog", { name: "编辑分流规则" });
  await editRoute.getByLabel("名称", { exact: true }).fill("规则 C2");
  await editRoute.getByRole("button", { name: "保存", exact: true }).click();
  await routing.getByText("规则 C2", { exact: true }).waitFor();
  await routing.getByRole("button", { name: "上移 规则 C2", exact: true }).click();
  check(await page.evaluate(() => window.fixture012.calls.some(x => x.command === 'mutate_proxy_routing' && x.args.request.mutation.type === 'reorderRoutes')), "route reorder mutation exact");
  await routing.getByRole("button", { name: "删除 规则 C2", exact: true }).click();
  check(await routing.getByText("规则 C2", { exact: true }).count() === 0, "route delete applies authoritative snapshot");

  await routing.getByRole("button", { name: "应用", exact: true }).click();
  await routing.getByText("应用中…", { exact: true }).waitFor();
  check(await routing.getByRole("button", { name: "新建规则", exact: true }).isDisabled(), "Apply pending disables conflicting controls");
  await routing.getByText("正在应用配置", { exact: true }).waitFor();
  await page.waitForFunction(() => window.fixture012.snapshot.applyState.type === "applied");
  await routing.getByText("应用中…", { exact: true }).waitFor({ state: "hidden", timeout: 3000 });
  check(await routing.getByRole("button", { name: "应用", exact: true }).isDisabled(), "Apply terminal clears dirty action");

  const queryCountBefore = await page.evaluate(() => window.fixture012.calls.filter(x => x.command === 'get_proxy_routing_snapshot').length);
  await page.evaluate(async () => { const f=window.fixture012; f.snapshot.revision++; f.snapshot.runtimeState='stopped'; f.snapshot.appliedSubscriptionId=null; f.snapshot.applyState={type:'savedPendingApply'}; f.snapshot.selectors=f.snapshot.selectors.map(s=>({...s,runtimeNodeId:null,state:'savedOnly'})); await f.emit('runtime_observation_delta',{source:'runtime',revision:2,observedAtMs:2,trafficHistory:[],captureMode:'off',sidecarLifecycle:'stopped',uploadRateBps:0,downloadRateBps:0,uploadTotalBytes:0,downloadTotalBytes:0,connectionCount:0,logSummary:[],appliedSubscriptionId:null,appliedConfigurationGeneration:null,subscriptionSwitch:null,managedProxyAvailable:false,coreMemoryBytes:null}); });
  await page.waitForFunction((before) => window.fixture012.calls.filter(x => x.command === 'get_proxy_routing_snapshot').length > before, queryCountBefore);
  await page.getByRole("button", { name: "出口组", exact: true }).click();
  await proxies.getByText("有未应用更改", { exact: true }).waitFor();
  check(await proxies.getByText("已保存", { exact: true }).isVisible(), "runtime event refreshes shared Provider for Proxies");
  await page.getByRole("button", { name: "分流", exact: true }).click();
  check(await routing.getByLabel("默认出口", { exact: true }).inputValue() === "block", "Routing consumes the same authoritative event-refreshed snapshot");

  const mutations = await page.evaluate(() => window.fixture012.calls.filter(x => x.command === 'mutate_proxy_routing').map(x => x.args.request.mutation.type));
  for (const required of ["createCustomPool", "updateCustomPool", "deleteCustomPool", "setManualSelection", "setDefaultTarget", "createRoute", "updateRoute", "deleteRoute", "reorderRoutes", "applyConfiguration"]) check(mutations.includes(required), `mutation matrix includes ${required}`);
  check(errors.length === 0, "browser page has no errors");
  return { checks, mutations, errors, queryCountBefore, queryCountAfter: await page.evaluate(() => window.fixture012.calls.filter(x => x.command === 'get_proxy_routing_snapshot').length), boundary: "Chromium with official Tauri mockIPC and synthetic Veyra domain fixtures; no native backend, persistence or core" };
}
