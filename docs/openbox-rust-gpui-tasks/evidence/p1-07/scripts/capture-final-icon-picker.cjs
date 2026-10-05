const {chromium}=require('/Users/lifeilin/.npm/_npx/e41f203b7505f1fb/node_modules/playwright');
const fs=require('fs');
(async()=>{
 const out=process.cwd()+'/docs/openbox-rust-gpui-tasks/evidence/p1-07/final-20261005/react';
 const browser=await chromium.launch({headless:true,executablePath:'/Users/lifeilin/Library/Caches/ms-playwright/chromium_headless_shell-1226/chrome-headless-shell-mac-arm64/chrome-headless-shell'});
 const audit=[],metrics=[];
 for(const theme of ['light','dark']){
  const data=JSON.parse(fs.readFileSync('docs/openbox-rust-gpui-tasks/fixtures/p0-01/visual-data.json')).mocks;
  Object.assign(data['/api/storage'].entries,{'config/theme-mode':theme,'config/speedtest-timeout':'6500','config/ipv6-test':'false','config/global-radius':'16','config/blur-intensity':'10','config/custom-background-image':''});
  data['/api/openbox/service/status'].core.running=false;
  const context=await browser.newContext({viewport:{width:1280,height:720},deviceScaleFactor:1});
  await context.route('**/*',async r=>{const u=new URL(r.request().url());if(u.hostname!=='127.0.0.1'){audit.push({blocked:u.href});return r.abort();}if(u.pathname.startsWith('/api/')){let v=data[u.pathname];if(r.request().method()==='PATCH'&&u.pathname==='/api/storage'){const patch=JSON.parse(r.request().postData()||'{}');Object.assign(data['/api/storage'].entries,patch.entries||{});v=data['/api/storage'];}audit.push({path:u.pathname,method:r.request().method()});return r.fulfill({contentType:'application/json',body:JSON.stringify(v??{})});}return r.continue();});
  await context.routeWebSocket('**/*',()=>{});
  const page=await context.newPage();
  const shot=async state=>{
   metrics.push({theme,state,entries:structuredClone(data['/api/storage'].entries),rects:await page.evaluate(()=>[...document.querySelectorAll('.sidebar,.sidebar-top,.brand,.sidebar-collapse-control,.sidebar-collapse-tooltip,.main-nav button,.sidebar-status,.sidebar-bottom,.settings-top-nav,.settings-nav-scroll button,.settings-content,.settings-block,.compact-setting,.panel-select,.ob-switch,.panel-range,.panel-ip-tooltip,.number-field input,[data-sonner-toast],[data-close-button],.ob-modal,.test-site-row')].map(e=>{const r=e.getBoundingClientRect(),s=getComputedStyle(e),b=getComputedStyle(e,'::before');return{selector:e.className,label:e.getAttribute('aria-label')||e.textContent.slice(0,40),x:r.x,y:r.y,w:r.width,h:r.height,color:s.color,background:s.backgroundColor,gap:s.gap,font:s.font,border:s.border,borderRadius:s.borderRadius,opacity:s.opacity,boxShadow:s.boxShadow,checked:e.getAttribute('aria-checked'),before:{width:b.width,height:b.height,transform:b.transform,background:b.backgroundColor,top:b.top,left:b.left,content:b.content}}}))});
   await page.screenshot({animations:'disabled',path:out+'/'+theme+'-'+state+'.png'});
  };
  await page.goto('http://127.0.0.1:1427/openbox.html#/settings');await page.waitForSelector('.panel-settings');await page.evaluate(()=>document.fonts.ready);await page.mouse.move(800,500);await page.getByRole('button',{name:'百度 图标',exact:true}).click();await shot('icon-picker');
  await context.close();
 }
 fs.writeFileSync(out+'/icon-picker-metrics.json',JSON.stringify(metrics,null,2));fs.writeFileSync(out+'/icon-picker-request-audit.json',JSON.stringify(audit,null,2));await browser.close();
})().catch(e=>{console.error(e);process.exitCode=1});
