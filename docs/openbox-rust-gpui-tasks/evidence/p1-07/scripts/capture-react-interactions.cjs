const {chromium}=require('/Users/lifeilin/.npm/_npx/e41f203b7505f1fb/node_modules/playwright');
const fs=require('fs');
(async()=>{
 const out=process.cwd()+'/docs/openbox-rust-gpui-tasks/evidence/p1-07/react';
 const browser=await chromium.launch({headless:true,executablePath:'/Users/lifeilin/Library/Caches/ms-playwright/chromium_headless_shell-1226/chrome-headless-shell-mac-arm64/chrome-headless-shell'});
 const audit=[]; const metrics=[];
 for(const theme of ['light','dark']){
  const data=JSON.parse(fs.readFileSync('docs/openbox-rust-gpui-tasks/fixtures/p0-01/visual-data.json')).mocks;
  data['/api/storage'].entries['config/theme-mode']=theme;
  data['/api/storage'].entries['config/speedtest-timeout']='6500';
  data['/api/storage'].entries['config/ipv6-test']='false';
  const context=await browser.newContext({viewport:{width:1280,height:720},deviceScaleFactor:1});
  await context.route('**/*',async r=>{const u=new URL(r.request().url());if(u.hostname!=='127.0.0.1'){audit.push({blocked:u.href});return r.abort();}if(u.pathname.startsWith('/api/')){let v=data[u.pathname];if(r.request().method()==='PATCH'&&u.pathname==='/api/storage'){const patch=JSON.parse(r.request().postData()||'{}');Object.assign(data['/api/storage'].entries,patch.entries||{});v=data['/api/storage'];}audit.push({path:u.pathname,method:r.request().method()});return r.fulfill({contentType:'application/json',body:JSON.stringify(v??{})});}return r.continue();});
  await context.routeWebSocket('**/*',()=>{});
  const page=await context.newPage(); await page.goto('http://127.0.0.1:1427/openbox.html#/settings'); await page.waitForSelector('.panel-settings');await page.evaluate(()=>document.fonts.ready);
  const shot=async state=>{
   metrics.push({theme,state,rects:await page.evaluate(()=>Object.fromEntries(['.panel-ip-tooltip','.ip-info-tooltip','[aria-label="IPv6 测试"]'].map(selector=>{const e=document.querySelector(selector);const r=e?.getBoundingClientRect();return [selector,e?{x:r.x,y:r.y,width:r.width,height:r.height,checked:e.getAttribute('aria-checked')}:null]})))});
   await page.screenshot({animations:'disabled',path:out+'/'+theme+'-'+state+'.png'});
  };
  await page.getByRole('combobox',{name:'IP信息API',exact:true}).click();await shot('ip-select');await page.keyboard.press('Escape');
  await page.getByLabel('IP信息API说明').hover();await shot('tooltip-hover');
  await page.getByRole('combobox',{name:'IP信息API',exact:true}).focus();await page.keyboard.press('Shift+Tab');await shot('tooltip-focus');await page.keyboard.press('Tab');
  await page.getByRole('spinbutton',{name:'测速超时',exact:true}).focus();await shot('input-focus');
  await page.getByRole('button',{name:'恢复默认测试站点',exact:true}).hover();await shot('icon-button-hover');
  await page.getByRole('button',{name:'恢复默认测试站点',exact:true}).click();await shot('modal-open');await page.keyboard.press('Escape');await shot('modal-focus-return');
  await page.getByRole('switch',{name:'IPv6 测试',exact:true}).focus();await shot('switch-focus-off');await page.keyboard.press('Space');await page.waitForFunction(()=>document.querySelector('[aria-label="IPv6 测试"]').getAttribute('aria-checked')==='true');await shot('switch-focus-on');await page.waitForTimeout(300);await shot('toast-generic');
  await context.close();
 }
 fs.writeFileSync(out+'/interaction-metrics.json',JSON.stringify(metrics,null,2));
 fs.writeFileSync(out+'/interaction-request-audit.json',JSON.stringify(audit,null,2));await browser.close();
})().catch(e=>{console.error(e);process.exitCode=1});
