# P0-01 离线消费 fixtures

[基线与限制](../../P0-01-baseline.md) · [任务验收](../../P0-feasibility.md#obg-p0-01)

五组 JSON 共 22 case：groups 3、config 6、dns 3、errors 4、streams 6。`cases[].value` 是被测数据；id/provenance/limit 是元数据，不发真实 API。errors 的 status/contentType/body 是传输外壳，streams value 是一帧，不含接收时间；禁止导入生产或对示例地址联网。

每个 case 独立标来源类别、源码 commit、文件行、脱敏与限制；observed=false 表示没有运行采样。测试样本缩减/包装/规范化、混合来源在 limit 说明。源码消费示例仅说明形状，不保证真实 wire/语义。完整 profile/backup 未采集，部分 profile 不冒充完整默认值。

脱敏：example.com/example.net/.invalid 与文档 IP 网段；名称/ID 用 fixture，时间固定；不含有效 password/token/uuid/share URL/raw node link/Cookie/processPath。嵌套 JSON 同样审核。FIXTURE_UNAUTHORIZED 是合成消费码。source-inventory/visual-inventory 是结构/历史图片元数据，均不算响应 fixture。

解析复现（仓库根，无新增依赖）：

```sh
python3 - <<'PY'
import json
from pathlib import Path
files = list(Path('docs/openbox-rust-gpui-tasks/fixtures/p0-01').glob('*.json'))
cases = [c for p in files for c in json.loads(p.read_text()).get('cases', [])]
assert len(cases) == 77
assert sum(c['provenance']['observed'] is False for c in cases) == 22
assert sum(c['provenance']['observed'] is True for c in cases) == 55
print(len(files), 'JSON files;', len(cases), 'cases')
PY
```

API AST 属性计数复现（已有 TypeScript 7.0.2，非构建/测试）：

```sh
node --input-type=module <<'JS'
import { API } from './node_modules/typescript/dist/api/sync/api.js';
const api = new API();
try {
  const snap = api.updateSnapshot({ openProjects: ['tsconfig.json'] });
  const sf = snap.getProjects()[0].program.getSourceFile('src/openbox/api/client.ts');
  const declaration = sf.statements.find(s => s.declarationList?.declarations.some(d => d.name.getText(sf) === 'api'));
  console.log(declaration.declarationList.declarations[0].initializer.properties.length);
  snap.dispose();
} finally { api.close(); }
JS
```

业务引用复现：同一 AST program 遍历全 src，收集从 api/client 导入 api（含 alias）的 PropertyAccessExpression，分离 *.test.*，逐一比对结构清单 methods 的 business/tests。源码变动后应核对指纹再刷新，不默认旧样本等于新版本。

本次新增 `observed-http.json`（13 case）与 `observed-ws.json`（4 case）：真实无凭据 GET 403 / WS 无帧失败观测，全部 observed=true。没有取得业务 DTO/流帧；`fieldShape` / `frames` 不冒充 wire 原文。所有字符串、动态字典键及非消费字段名删除，HTML 仅字节长度。共 9 个 fixture/元数据 JSON、39 case（22 离线 + 17 观测）；环境证据另见 [browser-attempt.json](../../evidence/p0-01/browser-attempt.json)，它不算响应 case。精确方法/命令/限制见 [本轮实际结果](../../P0-01-baseline.md#readonly-observation)。

追加 `observed-session-http.json` 16 case（前置 GET、仅一次登录 POST、批准的 13 GET、HTTP1.1 GET 确认）与 `observed-session-ws.json` 4 case：仍全部失败，无业务 DTO/帧。源码/测试样本 22 + observed 请求失败 37 = **59 cases**；fixture/元数据 JSON **11**，环境证据 JSON **2**，合计 **13**。仅登录 POST 例外；请求 body/headers/临时会话不留仓库，所有响应内容仅留脱敏形状。浏览器仍无截图；[完整方法与清理记录](../../P0-01-baseline.md#temporary-auth-observation)。此前计数属于历史批次，不与新增累加批次混淆。


当前最终验收新增 [observed-current-http.json](observed-current-http.json) 14 case（成功登录 + 13 指定读取），[observed-current-ws.json](observed-current-ws.json) 4 case（6/6/5/5 真实帧），均 observed=true。累计 **77 cases：22 静态 + 55 observed**；旧失败样本保持不变。全部字符串与动态键脱敏，数组缩减；traffic 数值为各字段排序秩，只保留比较模式，不是原单位、比例或使用量。控制器版本提取部分 1.14.1-openbox，服务版本 UNKNOWN。

[visual-data.json](visual-data.json) 为独立 observed=false 浏览器 mock（不计响应 case）；[视觉清单](../../evidence/p0-01/visual-manifest.json) 和[请求审计](../../evidence/p0-01/request-audit.json)记录当前真实 UI 与隔离边界。源码渲染真实，数据模拟；没有生产写操作。当前四项 PASS，P0-01 DONE；[完整结论及未知项](../../P0-01-baseline.md#current-acceptance)。
