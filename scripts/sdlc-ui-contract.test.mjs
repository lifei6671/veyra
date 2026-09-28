import { afterEach, beforeEach, describe, expect, it } from 'vitest';
import { mkdtempSync, mkdirSync, writeFileSync, readFileSync, existsSync, rmSync, renameSync, symlinkSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join, dirname } from 'node:path';
import { createHash } from 'node:crypto';
import { evaluateUiGate, CONTRACT } from './sdlc-ui-contract.mjs';

// Synthetic approvals are confined to disposable test roots, never project Evidence.
const TEST_CONTRACT_VERSION = '0.4';
let root;
const identity = 'sha256:test-target';
const hash = path => 'sha256:' + createHash('sha256').update(readFileSync(join(root, path))).digest('hex');
function write(path, value) {
  mkdirSync(dirname(join(root, path)), { recursive: true });
  writeFileSync(join(root, path), typeof value === 'string' ? value : JSON.stringify(value));
}
function metadata(page = 'Subscriptions', id = 'TASK-100') {
  return {
    task_id: id, scope: { allow: ['Update visual component layout'], deny: [] },
    ui_impact: { required: true, reason: 'Update page composition' },
    ui_contract: { document: CONTRACT, version: TEST_CONTRACT_VERSION, required: true },
    ui_scope: { pages: [page], components: ['PageHeader'],
      states: ['normal', 'hover', 'selected', 'disabled', 'loading', 'empty', 'error', 'busy', 'focus', 'success'],
      rules: ['§4 Typography'] },
    visual_gate: { required: true, human_approval: true },
  };
}
function gate(data, phase = 'design', extra = {}) {
  write('candidate.json', data);
  return evaluateUiGate({ root, artifact: 'candidate.json', phase, targetIdentity: identity, ...extra });
}
function compliance(data, baseOverride, targetOverride) {
  const target = targetOverride ?? identity;
  const base = baseOverride ?? '.sdlc/evidence/' + data.task_id + '/';
  for (const command of ['pnpm lint', 'pnpm test', 'pnpm build'])
    write(base + command.slice(5) + '.json', { command_or_method: command, result: 'PASS', exit_code: 0,
      target_identity: target, timestamp: '2026-09-07T00:00:00Z', observer: 'test-runner', summary: 'Synthetic verification fixture' });
  write(base + 'review.json', { result: 'PASS', target_identity: target, reviewer: 'independent-reviewer',
    command_or_method: 'Inspect CSS/token/composition against Contract', timestamp: '2026-09-07T00:00:00Z',
    summary: 'Synthetic independent review fixture', coverage: data.ui_scope });
  const shots = [[1280, 720, 100], [1440, 900, 100], [1280, 720, 125], [1440, 900, 125], [1280, 720, 150]].map(([width, height, scale], i) => {
    const theme = i ? 'dark' : 'light';
    const path = base + theme + '-' + i + '.png';
    write(path, 'synthetic screenshot fixture');
    return { path, sha256: hash(path), target_identity: target, source: 'tauri-windows',
      page: data.ui_scope.pages[0], states: data.ui_scope.states, theme,
      width, height, scale };
  });
  const value = { task_id: data.task_id, contract: CONTRACT, contract_version: data.ui_contract.version,
    target_identity: target, produced_by: 'producer', result: 'PASS', findings: [],
    checked_pages: data.ui_scope.pages, checked_components: data.ui_scope.components,
    checked_states: data.ui_scope.states, checked_rules: data.ui_scope.rules, screenshots: shots,
    verification: ['pnpm lint', 'pnpm test', 'pnpm build'].map(command => ({ command, exit_code: 0,
      result: 'PASS', target_identity: target, evidence_ref: base + command.slice(5) + '.json' })),
    review: { result: 'PASS', reviewer: 'independent-reviewer', evidence_ref: base + 'review.json' } };
  write(base + 'ui-contract-compliance.json', value);
  return value;
}
function humanFixture(data, value, baseOverride) {
  const base = baseOverride ?? '.sdlc/evidence/' + data.task_id + '/';
  const approval = { task_id: data.task_id, contract: CONTRACT, contract_version: data.ui_contract.version,
    target_identity: value.target_identity, approved_by: 'human', recorded_by: 'human', decision: 'APPROVED',
    timestamp: '2026-09-07T00:00:00Z', reviewed_screenshots: value.screenshots.map(x => x.path),
    compliance_sha256: hash(base + 'ui-contract-compliance.json') };
  write(base + 'ui-human-approval.json', approval);
  return approval;
}
function certification(data, { approved = false, stageId, golden = true, prerequisites = [], reference = false, evidenceRef } = {}) {
  const page = data.ui_scope.pages[0];
  const scope = { ...data.ui_scope, source_paths: data.ui_scope.source_paths ?? ['src/styles.css', 'src/' + page + '.tsx'] };
  for (const path of scope.source_paths) if (!existsSync(join(root, path))) write(path, 'fixture source: ' + path);
  const manifest = scope.source_paths.map(path => ({ path, sha256: hash(path) }));
  const target = 'sha256:' + createHash('sha256').update(JSON.stringify({ git_identity: 'a'.repeat(40), manifest })).digest('hex');
  const base = evidenceRef ? evidenceRef.slice(0, evidenceRef.lastIndexOf('/') + 1) :
    '.sdlc/evidence/' + (stageId ? data.task_id + '/ui-stages/' + stageId : 'golden-pages/' + page) + '/';
  const certId = 'GOLDEN-' + page + '-TEST';
  const value = compliance({ ...data, ui_scope: scope }, base, target);
  value.certification_kind = golden ? 'golden_page_certification' : 'ui_stage_acceptance';
  value.prerequisites = prerequisites;
  if (reference) {
    Object.assign(value, { verification_level: 'stage_reference', verification_model: 'stage_reference_then_final_matrix' });
    const shot = value.screenshots[0];
    shot.page = page; shot.theme = 'light'; shot.width = 1280; shot.height = 720; shot.scale = 150;
    shot.states = scope.states;
    value.screenshots = [shot];
  }
  const review = JSON.parse(readFileSync(join(root, value.review.evidence_ref), 'utf8'));
  review.certification_kind = value.certification_kind;
  review.prerequisites = prerequisites;
  if (reference) Object.assign(review, { task_id: data.task_id, stage_id: stageId, contract: CONTRACT,
    contract_version: data.ui_contract.version, verification_level: 'stage_reference',
    verification_model: 'stage_reference_then_final_matrix' });
  write(value.review.evidence_ref, review);
  if (stageId) value.stage_id = stageId;
  else { delete value.task_id; value.certification_id = certId; }
  write(base + 'ui-contract-compliance.json', value);
  const record = { kind: golden ? 'golden_page_certification' : 'ui_stage_acceptance',
    ...(stageId ? { task_id: data.task_id, stage_id: stageId } : { certification_id: certId }),
    page, contract: CONTRACT, contract_version: data.ui_contract.version, ui_scope: scope, target_identity: target,
    git_identity: 'a'.repeat(40), manifest, prerequisites,
    compliance: { path: base + 'ui-contract-compliance.json', sha256: hash(base + 'ui-contract-compliance.json') } };
  if (reference) Object.assign(record, { verification_level: 'stage_reference', verification_model: 'stage_reference_then_final_matrix' });
  if (approved) {
    const approval = humanFixture(data, value, base);
    if (stageId) approval.stage_id = stageId;
    else { delete approval.task_id; approval.certification_id = certId; }
    if (reference) Object.assign(approval, { verification_level: 'stage_reference', verification_model: 'stage_reference_then_final_matrix' });
    write(base + 'ui-human-approval.json', approval);
    record.human_approval = { path: base + 'ui-human-approval.json', sha256: hash(base + 'ui-human-approval.json') };
  }
  const path = evidenceRef ?? base + 'certification.json'; write(path, record);
  return { path, record, value, base };
}
function stagedTask() {
  const data = metadata('Proxies', 'TASK-012');
  data.ui_scope.pages.push('Routing');
  data.ui_stages = ['Proxies', 'Routing'].map((page, index) => ({
    id: index ? 'B' : 'A',
    ui_scope: { ...metadata(page).ui_scope, source_paths: ['src/styles.css', 'src/' + page + '.tsx'] },
    requires_golden_pages: [index ? 'Proxies' : 'Subscriptions'],
    ...(index ? {} : { produces_golden_page: 'Proxies' }),
    evidence_ref: '.sdlc/evidence/TASK-012/ui-stages/' + (index ? 'B' : 'A') + '/certification.json',
  }));
  const external = certification(metadata('Subscriptions'), { approved: true });
  data.ui_scope.golden_pages = [{ page: 'Subscriptions', certification_ref: external.path }];
  return data;
}
function stageProof(data, id, approved = true) {
  const item = data.ui_stages.find(item => item.id === id);
  const prerequisites = item.requires_golden_pages.map(page => {
    const certification_ref = data.ui_stages.find(other => other.produces_golden_page === page)?.evidence_ref ??
      data.ui_scope.golden_pages.find(other => other.page === page).certification_ref;
    return { page, certification_ref, sha256: hash(certification_ref) };
  });
  return certification({ ...data, ui_scope: item.ui_scope }, { stageId: id, approved, golden: !!item.produces_golden_page, prerequisites });
}
function twoLevelTask() {
  const version = '0.5';
  const standardStates = ['normal', 'hover', 'selected', 'disabled', 'loading', 'empty', 'error', 'busy', 'focus', 'success'];
  const components = ['PageHeader', 'Toast', 'RecoveryAction'];
  const rules = ['§4 Typography', '§6.7 Feedback'];
  const sources = ['src/shared.css', 'src/Subscriptions.tsx', 'src/Proxies.tsx', 'src/Routing.tsx'];
  const data = metadata('Subscriptions', 'TASK-019');
  data.ui_contract.version = version;
  data.ui_scope = { pages: ['Subscriptions', 'Proxies', 'Routing'], components,
    states: [...standardStates, 'toast-persistent'], rules, source_paths: sources };
  data.ui_stages = [
    { id: 'S', page: 'Subscriptions', requires: [], produces: 'Subscriptions', states: standardStates,
      paths: ['src/shared.css', 'src/Subscriptions.tsx'] },
    { id: 'A', page: 'Proxies', requires: ['Subscriptions'], produces: 'Proxies', states: [...standardStates, 'toast-persistent'],
      paths: ['src/shared.css', 'src/Proxies.tsx'] },
    { id: 'B', page: 'Routing', requires: ['Proxies'], states: [...standardStates, 'toast-persistent'],
      paths: ['src/shared.css', 'src/Routing.tsx'] },
  ].map(item => ({ id: item.id, ui_scope: { pages: [item.page], components, states: item.states, rules, source_paths: item.paths },
    requires_golden_pages: item.requires, ...(item.produces ? { produces_golden_page: item.produces } : {}),
    evidence_ref: `.sdlc/evidence/TASK-019/ui-stages/${item.id}/certification-001.json` }));
  data.ui_verification = { model: 'stage_reference_then_final_matrix', final_evidence: {
    compliance_ref: '.sdlc/evidence/TASK-019/final-visual/compliance-001.json',
    human_approval_ref: '.sdlc/evidence/TASK-019/final-visual/human-approval-001.json' } };
  return data;
}
function referenceProof(data, id, approved = true) {
  const item = data.ui_stages.find(stage => stage.id === id);
  const prerequisites = item.requires_golden_pages.map(page => {
    const producer = data.ui_stages.find(stage => stage.produces_golden_page === page);
    return { page, certification_ref: producer.evidence_ref, sha256: hash(producer.evidence_ref) };
  });
  return certification({ ...data, ui_scope: item.ui_scope }, { stageId: id, approved,
    golden: !!item.produces_golden_page, prerequisites, reference: true, evidenceRef: item.evidence_ref });
}
function allReferenceProofs(data) {
  return data.ui_stages.map(item => referenceProof(data, item.id));
}
function finalProof(data, { approved = false, reverseManifest = false } = {}) {
  for (const path of data.ui_scope.source_paths) if (!existsSync(join(root, path))) write(path, 'fixture source: ' + path);
  const manifestPaths = reverseManifest ? [...data.ui_scope.source_paths].reverse() : data.ui_scope.source_paths;
  const manifest = manifestPaths.map(path => ({ path, sha256: hash(path) }));
  const git_identity = 'f'.repeat(40);
  const target = 'sha256:' + createHash('sha256').update(JSON.stringify({ git_identity, manifest })).digest('hex');
  const compliancePath = data.ui_verification.final_evidence.compliance_ref;
  const base = compliancePath.slice(0, compliancePath.lastIndexOf('/') + 1);
  const value = compliance(data, base, target);
  const screenshots = [];
  for (const page of data.ui_scope.pages) for (const theme of ['light', 'dark'])
    for (const [width, height, scale] of [[1280, 720, 100], [1280, 720, 125], [1280, 720, 150], [1440, 900, 100], [1440, 900, 125]]) {
      const path = `${base}${page}-${theme}-${width}x${height}-${scale}.png`;
      write(path, 'synthetic final screenshot fixture');
      screenshots.push({ path, sha256: hash(path), target_identity: target, source: 'tauri-windows', page,
        states: ['normal'], theme, width, height, scale });
    }
  const stage_references = data.ui_stages.map(item => ({ stage_id: item.id, path: item.evidence_ref, sha256: hash(item.evidence_ref) }));
  const review = JSON.parse(readFileSync(join(root, value.review.evidence_ref), 'utf8'));
  Object.assign(review, { task_id: data.task_id, contract: CONTRACT, contract_version: data.ui_contract.version,
    verification_level: 'task_final_matrix', verification_model: 'stage_reference_then_final_matrix', stage_references });
  write(value.review.evidence_ref, review);
  Object.assign(value, { verification_level: 'task_final_matrix', verification_model: 'stage_reference_then_final_matrix',
    git_identity, manifest, stage_references, screenshots });
  value.review.sha256 = hash(value.review.evidence_ref);
  write(compliancePath, value);
  if (approved) {
    const approvalPath = data.ui_verification.final_evidence.human_approval_ref;
    write(approvalPath, { task_id: data.task_id, contract: CONTRACT, contract_version: data.ui_contract.version,
      verification_level: 'task_final_matrix', verification_model: 'stage_reference_then_final_matrix',
      target_identity: target, approved_by: 'human', recorded_by: 'human', decision: 'APPROVED',
      timestamp: '2026-09-07T00:00:00Z', reviewed_screenshots: screenshots.map(shot => shot.path),
      compliance_sha256: hash(compliancePath) });
  }
  return { value, target, compliancePath, base };
}
beforeEach(() => {
  root = mkdtempSync(join(tmpdir(), 'veyra-ui-gate-test-'));
  write(CONTRACT, `---\nVersion: "${TEST_CONTRACT_VERSION}"\nStatus: APPROVED\nContract: BINDING\n---\n`);
});
afterEach(() => rmSync(root, { recursive: true, force: true }));

describe('Veyra UI delivery gates', () => {
  it('Task readiness checks only external certification, not its own future Proxies stage', () => {
    const data = stagedTask(); write('design.json', data);
    expect(gate(data, 'readiness', { design: 'design.json' }).result).toBe('PASS');
    expect(gate(data, 'checkpoint', { stage: 'A' }).result).toBe('PASS');
    expect(gate(data, 'checkpoint', { stage: 'B' }).result).toBe('FAIL');
  });
  it('Routing checkpoint requires Proxies stage human approval, not merely compliance', () => {
    const data = stagedTask(); stageProof(data, 'A', false);
    expect(gate(data, 'compliance', { stage: 'A' }).result).toBe('PASS');
    expect(gate(data, 'checkpoint', { stage: 'B' }).result).toBe('FAIL');
    stageProof(data, 'A');
    expect(gate(data, 'checkpoint', { stage: 'B' }).result).toBe('PASS');
  });
  it('final delivery requires every stage and final whole-Task human approval', () => {
    const data = stagedTask(); stageProof(data, 'A');
    const whole = compliance(data);
    whole.screenshots.push(...whole.screenshots.map(shot => ({ ...shot, page: 'Routing' })));
    write('.sdlc/evidence/TASK-012/ui-contract-compliance.json', whole);
    expect(gate(data, 'delivery').result).toBe('FAIL');
    stageProof(data, 'B', false);
    expect(gate(data, 'delivery').result).toBe('FAIL');
    stageProof(data, 'B');
    expect(gate(data, 'delivery').result).toBe('FAIL');
    humanFixture(data, whole);
    expect(gate(data, 'delivery').result).toBe('PASS');
  });
  it('unrelated Routing source changes preserve Proxies certification; shared CSS changes stale it', () => {
    const data = stagedTask(); stageProof(data, 'A');
    write('src/Routing.tsx', 'new Routing layout');
    expect(gate(data, 'checkpoint', { stage: 'B' }).result).toBe('PASS');
    write('src/styles.css', 'changed shared token');
    expect(gate(data, 'checkpoint', { stage: 'B' }).result).toBe('FAIL');
  });
  it('standalone certification supports compliance before human approval and never needs TASK-011', () => {
    const cert = certification(metadata());
    expect(evaluateUiGate({ root, artifact: cert.path, phase: 'compliance' }).result).toBe('PASS');
    expect(evaluateUiGate({ root, artifact: cert.path, phase: 'certification' }).result).toBe('FAIL');
    const approved = certification(metadata(), { approved: true });
    expect(evaluateUiGate({ root, artifact: approved.path, phase: 'certification' }).result).toBe('PASS');
  });
  it.each(['contract_version', 'target_identity', 'git_identity'])('invalid certification %s fails', field => {
    const cert = certification(metadata(), { approved: true }); cert.record[field] = 'wrong'; write(cert.path, cert.record);
    expect(evaluateUiGate({ root, artifact: cert.path, phase: 'certification' }).result).toBe('FAIL');
  });
  it('certification checks compliance and human record hashes', () => {
    const cert = certification(metadata(), { approved: true });
    write(cert.record.compliance.path, { ...cert.value, produced_by: 'changed' });
    expect(evaluateUiGate({ root, artifact: cert.path, phase: 'certification' }).result).toBe('FAIL');
    const fresh = certification(metadata(), { approved: true }); write(fresh.record.human_approval.path, '{}');
    expect(evaluateUiGate({ root, artifact: fresh.path, phase: 'certification' }).result).toBe('FAIL');
  });
  it('stages cannot be reordered, omitted, or removed during materialization', () => {
    const data = stagedTask(); write('design.json', data);
    const reversed = { ...data, ui_stages: [...data.ui_stages].reverse() };
    expect(gate(reversed, 'design').result).toBe('FAIL');
    const omitted = { ...data, ui_stages: [data.ui_stages[0]] };
    expect(gate(omitted, 'design').result).toBe('FAIL');
    const removed = { ...data }; delete removed.ui_stages;
    expect(gate(removed, 'readiness', { design: 'design.json' }).result).toBe('FAIL');
  });
  it('stage selector is mandatory for staged implementation checkpoints', () => {
    const data = stagedTask(); expect(gate(data, 'checkpoint').result).toBe('FAIL');
    expect(gate(data, 'checkpoint', { stage: 'unknown' }).result).toBe('FAIL');
  });
  it('a certification cannot masquerade as a Task or implementation permission', () => {
    const cert = certification(metadata(), { approved: true });
    expect(evaluateUiGate({ root, artifact: cert.path, phase: 'readiness' }).result).toBe('FAIL');
    expect(gate(metadata(), 'certification').result).toBe('FAIL');
  });
  it('same-Task evidence cannot be labelled an external Golden Page prerequisite', () => {
    const cert = certification(metadata('Subscriptions', 'TASK-012'), { approved: true, stageId: 'hidden' });
    const data = metadata('Proxies', 'TASK-012');
    data.ui_scope.golden_pages = [{ page: 'Subscriptions', certification_ref: cert.path }];
    write('design.json', data);
    expect(gate(data, 'readiness', { design: 'design.json' }).result).toBe('FAIL');
  });
  it('another Task established Golden Page can be reused without reading its historical Task', () => {
    const upstream = certification(metadata(), { approved: true });
    const cert = certification(metadata('Proxies', 'TASK-100'), { approved: true, stageId: 'A',
      prerequisites: [{ page: 'Subscriptions', certification_ref: upstream.path, sha256: hash(upstream.path) }] });
    const data = metadata('Connections', 'TASK-200');
    data.ui_scope.golden_pages = [{ page: 'Proxies', certification_ref: cert.path }];
    write('design.json', data);
    expect(gate(data, 'readiness', { design: 'design.json' }).result).toBe('PASS');
    expect(existsSync(join(root, '.sdlc/tasks'))).toBe(false);
  });
  it.each([undefined, 'A'])('Proxies certification cannot omit Subscriptions lineage (stage %s)', stageId => {
    const cert = certification(metadata('Proxies'), { approved: true, stageId });
    expect(evaluateUiGate({ root, artifact: cert.path, phase: 'certification' }).result).toBe('FAIL');
    const data = metadata('Connections', 'TASK-200');
    data.ui_scope.golden_pages = [{ page: 'Proxies', certification_ref: cert.path }]; write('design.json', data);
    expect(gate(data, 'readiness', { design: 'design.json' }).result).toBe('FAIL');
  });
  it('upstream certification changes invalidate its bound downstream lineage', () => {
    const data = stagedTask(); stageProof(data, 'A');
    const path = data.ui_scope.golden_pages[0].certification_ref;
    const upstream = JSON.parse(readFileSync(join(root, path), 'utf8'));
    write(path, { ...upstream, note: 'different certification record' });
    expect(gate(data, 'checkpoint', { stage: 'B' }).result).toBe('FAIL');
  });
  it('a normal stage cannot promote its envelope to Golden without new compliance and human approval', () => {
    const data = stagedTask(); stageProof(data, 'A');
    const normal = stageProof(data, 'B'); normal.record.kind = 'golden_page_certification'; write(normal.path, normal.record);
    expect(gate(data, 'delivery', { stage: 'B' }).result).toBe('FAIL');
    // A Proxies stage without producer status cannot be promoted and reused externally either.
    const proxy = certification(metadata('Proxies', 'TASK-100'), { stageId: 'ordinary', approved: true, golden: false });
    proxy.record.kind = 'golden_page_certification'; write(proxy.path, proxy.record);
    const consumer = metadata('Connections', 'TASK-200');
    consumer.ui_scope.golden_pages = [{ page: 'Proxies', certification_ref: proxy.path }]; write('design.json', consumer);
    expect(gate(consumer, 'readiness', { design: 'design.json' }).result).toBe('FAIL');
  });
  it('changing a valid-looking Git identity stales certification', () => {
    const cert = certification(metadata(), { approved: true }); cert.record.git_identity = 'b'.repeat(40); write(cert.path, cert.record);
    expect(evaluateUiGate({ root, artifact: cert.path, phase: 'certification' }).result).toBe('FAIL');
  });
  it('non-UI Task needs no UI Contract, including frontend-only data work', () => {
    for (const phase of ['design', 'readiness', 'checkpoint', 'delivery'])
      expect(gate({ scope: { allow: ['src/parser.tsx DTO parsing'], deny: ['Sidebar'] } }, phase)).toMatchObject({ result: 'PASS', applicable: false });
  });
  it('UI Task without UI Contract fails Design Review', () => {
    const data = metadata(); delete data.ui_contract;
    expect(gate(data).result).toBe('FAIL');
  });
  it('legacy manifest Candidate without any UI metadata cannot bypass Design Review', () => {
    write('.sdlc/design/page.md', '# Design\n\n## 2. 产品与 UI 边界\n\n### 页面 Composition\nPageHeader and dense list\n\n## 3. IPC\n');
    const candidate = { status: 'CANDIDATE', task: 'TASK-100', manifest: [{ path: '.sdlc/design/page.md', sha256: 'historical' }] };
    expect(gate(candidate)).toMatchObject({ result: 'FAIL', applicable: true });
  });
  it('non-UI manifest Candidate keeps existing behavior', () => {
    write('.sdlc/design/parser.md', '# Parser\n\n## Scope\n\n### allow\nDTO validation\n\n### deny\nSidebar\n');
    expect(gate({ manifest: [{ path: '.sdlc/design/parser.md' }] })).toMatchObject({ result: 'PASS', applicable: false });
  });
  it.each(['0.1', 0.2, '0.3'])('wrong contract version %s fails', version => {
    const data = metadata(); data.ui_contract.version = version;
    expect(gate(data).result).toBe('FAIL');
  });
  it('valid UI Contract permits design and materialization gates', () => {
    const data = metadata();
    expect(gate(data).result).toBe('PASS');
    write('design.json', data);
    expect(gate(data, 'readiness', { design: 'design.json' }).result).toBe('PASS');
  });
  it('implementation complete without compliance blocks Delivery', () => {
    expect(gate({ ...metadata(), implementation_status: 'IMPLEMENTED' }, 'delivery').result).toBe('FAIL');
  });
  it('compliance PASS without human approval cannot complete Delivery', () => {
    const data = metadata(); compliance(data);
    expect(gate(data, 'delivery').result).toBe('FAIL');
  });
  it('current human APPROVED evidence permits the UI portion of Delivery', () => {
    const data = metadata(); humanFixture(data, compliance(data));
    expect(gate(data, 'delivery').result).toBe('PASS');
  });
  it('the Binding Spec human evidence example matches the executable contract', () => {
    const spec = readFileSync(new URL('../docs/ui/veyra-ui-spec.md', import.meta.url), 'utf8');
    const section = spec.split('### 11.4 Human Visual Approval')[1].split('## 12.')[0];
    const example = JSON.parse(section.match(/```json\s*([\s\S]*?)```/)[1]);
    const data = metadata(); const value = compliance(data); const actual = humanFixture(data, value);
    expect(Object.keys(example).sort()).toEqual(Object.keys(actual).sort());
    for (const key of ['contract', 'decision', 'approved_by', 'recorded_by'])
      expect(example[key]).toBe(actual[key]);
    expect(example.contract_version).toBe('0.5');
    expect(actual.contract_version).toBe(TEST_CONTRACT_VERSION);
    expect(gate(data, 'delivery').result).toBe('PASS');
  });
  it('agent-authored approval is rejected and evaluator never writes approval', () => {
    const data = metadata(); const approval = humanFixture(data, compliance(data));
    approval.recorded_by = 'agent';
    write('.sdlc/evidence/TASK-100/ui-human-approval.json', approval);
    expect(gate(data, 'delivery').result).toBe('FAIL');
    expect(readFileSync(new URL('./sdlc-ui-contract.mjs', import.meta.url), 'utf8')).not.toMatch(/writeFile|appendFile|mkdir|renameSync/);
  });
  it('missing Golden Page approval blocks Proxies readiness even with omitted dependency', () => {
    const data = metadata('Proxies'); write('design.json', data);
    expect(gate(data, 'readiness', { design: 'design.json' }).findings.join()).toContain('Golden Page');
  });
  it.each(['proxies', 'Proxies ', 'ProxyPage'])('noncanonical page %s cannot bypass Golden Page prerequisites', page => {
    const data = metadata(page); write('design.json', data);
    expect(gate(data, 'readiness', { design: 'design.json' }).result).toBe('FAIL');
  });
  it('approved Subscriptions certification unlocks Proxies without any historical Task', () => {
    const cert = certification(metadata(), { approved: true });
    const data = metadata('Proxies', 'TASK-101');
    data.ui_scope.golden_pages = [{ page: 'Subscriptions', certification_ref: cert.path }];
    write('design.json', data);
    expect(gate(data, 'readiness', { design: 'design.json' }).result).toBe('PASS');
    expect(existsSync(join(root, '.sdlc/tasks'))).toBe(false);
  });
  it.each(['ui_scope', 'visual_gate'])('missing %s fails design', field => {
    const data = metadata(); delete data[field]; expect(gate(data).result).toBe('FAIL');
  });
  it('missing or unapproved canonical Contract fails', () => {
    rmSync(join(root, CONTRACT)); expect(gate(metadata()).result).toBe('FAIL');
    write(CONTRACT, `Version: "${TEST_CONTRACT_VERSION}"\nStatus: DRAFT\nContract: BINDING\n`);
    expect(gate(metadata()).result).toBe('FAIL');
  });
  it('materialization cannot drop Design scope', () => {
    const data = metadata(); write('design.json', data); data.ui_scope.components.push('Notice');
    expect(gate(data, 'readiness', { design: 'design.json' }).result).toBe('FAIL');
  });
  it('materialization may reorder JSON keys without changing the Contract', () => {
    const data = metadata(); write('design.json', data);
    data.ui_contract = { required: true, version: TEST_CONTRACT_VERSION, document: CONTRACT };
    expect(gate(data, 'readiness', { design: 'design.json' }).result).toBe('PASS');
  });
  it('missing frontend verification or state coverage blocks delivery', () => {
    const data = metadata(); const value = compliance(data); value.verification.pop();
    write('.sdlc/evidence/TASK-100/ui-contract-compliance.json', value); humanFixture(data, value);
    expect(gate(data, 'delivery').result).toBe('FAIL');
    const next = compliance(data); next.checked_states = [];
    write('.sdlc/evidence/TASK-100/ui-contract-compliance.json', next); humanFixture(data, next);
    expect(gate(data, 'delivery').result).toBe('FAIL');
  });
  it.each(['lint', 'test', 'build', 'review'])('empty self-asserted %s reference cannot prove verification', check => {
    const data = metadata(); humanFixture(data, compliance(data));
    write('.sdlc/evidence/TASK-100/' + check + '.json', { result: 'PASS', target_identity: identity });
    expect(gate(data, 'delivery').result).toBe('FAIL');
  });
  it('changed screenshot, target or compliance invalidates approval', () => {
    const data = metadata(); const value = compliance(data); humanFixture(data, value);
    expect(gate(data, 'delivery', { targetIdentity: 'new-target' }).result).toBe('FAIL');
    write(value.screenshots[0].path, 'changed');
    expect(gate(data, 'delivery').result).toBe('FAIL');
    const current = compliance(data); humanFixture(data, current); current.produced_by = 'other';
    write('.sdlc/evidence/TASK-100/ui-contract-compliance.json', current);
    expect(gate(data, 'delivery').result).toBe('FAIL');
  });
  it.each([0, 1, 2, 3, 4])('missing mandatory DPI/window matrix entry %s blocks delivery', index => {
    const data = metadata(); const value = compliance(data); value.screenshots.splice(index, 1);
    write('.sdlc/evidence/TASK-100/ui-contract-compliance.json', value); humanFixture(data, value);
    expect(gate(data, 'delivery').result).toBe('FAIL');
  });
  it('page-specific states need not be fabricated on other pages', () => {
    const data = metadata('Subscriptions'); data.ui_scope.pages.push('Settings'); data.ui_scope.states.push('imported');
    const value = compliance(data);
    value.screenshots.push(...value.screenshots.map(shot => ({ ...shot, page: 'Settings', states: ['normal'] })));
    write('.sdlc/evidence/TASK-100/ui-contract-compliance.json', value); humanFixture(data, value);
    expect(gate(data, 'delivery').result).toBe('PASS');
  });
  it('Markdown deny/history does not falsely identify non-UI work', () => {
    write('task.md', '---\nid: TASK-100\n---\n## Scope\n\n### allow\n- DTO parsing\n\n### deny\n- Sidebar\n\n## History\nPage layout\n');
    expect(evaluateUiGate({ root, artifact: 'task.md', phase: 'design' }).applicable).toBe(false);
  });
});

// Shared delivery retains full blast-radius coverage; only native page matrices are representative.
function sharedMetadata() {
  const data = metadata();
  data.ui_scope.pages = ['Overview', 'Subscriptions', 'Proxies', 'Routing', 'Settings', 'Connections', 'Logs'];
  Object.assign(data.ui_scope, { delivery_mode: 'cross_cutting_shared', shared_components: ['PageHeader'],
    source_paths: ['src/shared.tsx'], verification_pages: ['Overview', 'Subscriptions', 'Proxies', 'Routing'], shared_review: 'shared-review.json' });
  qualifyShared(data);
  return data;
}
const orderScope = value => Array.isArray(value) ? value.map(orderScope) : value && typeof value === 'object' ? Object.fromEntries(Object.keys(value).sort().map(key => [key, orderScope(value[key])])) : value;
function qualifyShared(data, overrides = {}) {
  const { shared_review, ...ui_scope } = data.ui_scope;
  const scope_identity = 'sha256:' + createHash('sha256').update(JSON.stringify(orderScope({ task_id: data.task_id, scope: data.scope, ui_scope }))).digest('hex');
  write(shared_review, { result: 'PASS', delivery_mode: 'cross_cutting_shared', page_migration: false, scope_identity,
    reviewer: 'independent-reviewer', produced_by: 'producer', timestamp: '2026-09-08T00:00:00Z', summary: 'Synthetic shared-only classification, not a page migration', ...overrides });
}
function sharedCompliance(data) {
  const value = compliance(data);
  value.screenshots = ['light', 'dark'].flatMap(theme => [100, 125, 150].map(scale => ({
    ...value.screenshots[0], theme, scale, components: data.ui_scope.shared_components,
  })));
  value.screenshots.push(...data.ui_scope.verification_pages.slice(1).map(page => ({ ...value.screenshots[0], page })));
  write('.sdlc/evidence/TASK-100/ui-contract-compliance.json', value);
  return value;
}
describe('cross-cutting shared UI mode', () => {
  it('default and explicit migration keep Golden enforcement', () => {
    const data = sharedMetadata(); delete data.ui_scope.delivery_mode;
    expect(gate(data).findings.join()).toContain('Intra-task Golden');
    data.ui_scope.delivery_mode = 'page_migration';
    expect(gate(data).findings.join()).toContain('Intra-task Golden');
    expect(gate(metadata('Proxies'), 'checkpoint').findings.join()).toContain('External Golden');
  });
  it('full affected pages need no invented Golden stages or external prerequisites', () => {
    const data = sharedMetadata(); write('design.json', data);
    expect(gate(data).result).toBe('PASS');
    expect(gate(data, 'readiness', { design: 'design.json' }).result).toBe('PASS');
    expect(gate(data, 'checkpoint').result).toBe('PASS');
  });
  it.each(['ui_stages', 'produces_golden_page'])('rejects shared %s', key => {
    const data = sharedMetadata(); data[key] = key === 'ui_stages' ? [] : 'Subscriptions';
    expect(gate(data).result).toBe('FAIL');
  });
  it('rejects self/external Golden bindings and certification', () => {
    const data = sharedMetadata(); data.ui_scope.golden_pages = [{ page: 'Subscriptions', certification_ref: 'self.json' }];
    expect(gate(data).result).toBe('FAIL');
    delete data.ui_scope.golden_pages;
    data.kind = 'golden_page_certification'; data.page = 'Subscriptions';
    expect(gate(data, 'certification').findings.join()).toContain('cannot certify');
  });
  it('does not accept the mode flag alone, producer review or a rejected migration classification', () => {
    const data = sharedMetadata(); write('shared-review.json', {});
    expect(gate(data).result).toBe('FAIL');
    qualifyShared(data, { reviewer: 'producer' }); expect(gate(data).result).toBe('FAIL');
    qualifyShared(data, { page_migration: true }); expect(gate(data).result).toBe('FAIL');
    qualifyShared(data, { result: 'FAIL' }); expect(gate(data).result).toBe('FAIL');
  });
  it('binds classification to write scope and the complete affected/verification scope', () => {
    const data = sharedMetadata(); data.scope.allow.push('Migrate Routing page');
    expect(gate(data).result).toBe('FAIL');
    const other = sharedMetadata(); other.ui_scope.pages.pop();
    expect(gate(other).result).toBe('FAIL');
    const reduced = sharedMetadata(); reduced.ui_scope.verification_pages.pop();
    expect(gate(reduced).result).toBe('FAIL');
  });
  it.each(['verification_pages', 'shared_components', 'source_paths'])('requires nonempty %s', key => {
    const data = sharedMetadata(); data.ui_scope[key] = []; qualifyShared(data);
    expect(gate(data).result).toBe('FAIL');
  });
  it('rejects invalid representative page and unknown mode', () => {
    const data = sharedMetadata(); data.ui_scope.verification_pages = ['not-a-page']; qualifyShared(data);
    expect(gate(data).result).toBe('FAIL');
    data.ui_scope.delivery_mode = 'skip'; expect(gate(data).result).toBe('FAIL');
  });
  it('does not lose mode metadata on materialization', () => {
    const data = sharedMetadata(); write('design.json', data);
    data.ui_scope.verification_pages = ['Overview']; qualifyShared(data);
    expect(gate(data, 'readiness', { design: 'design.json' }).result).toBe('FAIL');
  });
  it('requires real component/context evidence, verification and human approval', () => {
    const data = sharedMetadata(); expect(gate(data, 'compliance').result).toBe('FAIL');
    const proof = sharedCompliance(data);
    expect(gate(data, 'compliance').result).toBe('PASS');
    expect(gate(data, 'delivery').result).toBe('FAIL');
    humanFixture(data, proof); expect(gate(data, 'delivery').result).toBe('PASS');
    proof.verification = []; write('.sdlc/evidence/TASK-100/ui-contract-compliance.json', proof);
    expect(gate(data, 'compliance').result).toBe('FAIL');
  });
  it.each(['context', 'component', 'theme-dpi', 'state', 'blast-radius', 'native', 'hash'])('rejects missing shared %s evidence', what => {
    const data = sharedMetadata(); const proof = sharedCompliance(data);
    if (what === 'context') proof.screenshots = proof.screenshots.filter(s => s.page !== 'Routing');
    if (what === 'component') proof.screenshots.forEach(s => { s.components = []; });
    if (what === 'theme-dpi') proof.screenshots = proof.screenshots.filter(s => !(s.theme === 'dark' && s.scale === 125));
    if (what === 'state') proof.screenshots.forEach(s => { s.states = ['normal']; });
    if (what === 'blast-radius') proof.checked_pages = data.ui_scope.verification_pages;
    if (what === 'native') proof.screenshots[0].source = 'mock';
    if (what === 'hash') proof.screenshots[0].sha256 = 'wrong';
    write('.sdlc/evidence/TASK-100/ui-contract-compliance.json', proof);
    expect(gate(data, 'compliance').result).toBe('FAIL');
  });
});

it('shared classification survives equivalent object key reorder during materialization', () => {
 const data = sharedMetadata(); write('design.json', data);
 const reverseKeys = x => Array.isArray(x) ? x.map(reverseKeys) : x && typeof x === 'object' ? Object.fromEntries(Object.entries(x).reverse().map(([k,v]) => [k,reverseKeys(v)])) : x;
 expect(gate(reverseKeys(data), 'readiness', { design: 'design.json' }).result).toBe('PASS');
});

const finalCells = ['Subscriptions', 'Proxies', 'Routing'].flatMap(page => ['light', 'dark'].flatMap(theme =>
  [[1280, 720, 100], [1280, 720, 125], [1280, 720, 150], [1440, 900, 100], [1440, 900, 125]]
    .map(([width, height, scale]) => ({ page, theme, width, height, scale }))));

describe('TASK-019 Contract0.5 two-level verification model', () => {
  const setup = () => {
    write(CONTRACT, '---\nVersion: "0.5"\nStatus: APPROVED\nContract: BINDING\n---\n');
    return twoLevelTask();
  };
  it('accepts only the exact TASK-019 model/stage shape and materializes all verification metadata', () => {
    const data = setup(); expect(gate(data, 'design').result).toBe('PASS');
    write('design.json', data); expect(gate(data, 'readiness', { design: 'design.json' }).result).toBe('PASS');
    const changed = structuredClone(data); changed.ui_verification.final_evidence.compliance_ref =
      '.sdlc/evidence/TASK-019/final-visual/compliance-002.json';
    expect(gate(changed, 'readiness', { design: 'design.json' }).result).toBe('FAIL');
  });
  it('allows stage compliance before Human but requires current Human for delivery and downstream checkpoint', () => {
    const data = setup(); referenceProof(data, 'S', false);
    expect(gate(data, 'compliance', { stage: 'S' }).result).toBe('PASS');
    expect(gate(data, 'delivery', { stage: 'S' }).result).toBe('FAIL');
    expect(gate(data, 'checkpoint', { stage: 'A' }).result).toBe('FAIL');
    referenceProof(data, 'S', true);
    expect(gate(data, 'delivery', { stage: 'S' }).result).toBe('PASS');
    expect(gate(data, 'checkpoint', { stage: 'A' }).result).toBe('PASS');
  });
  it.each(['theme', 'scale', 'viewport', 'native', 'target', 'hash', 'state'])('rejects reference screenshot with wrong %s', field => {
    const data = setup(); const proof = referenceProof(data, 'S', false);
    const shot = proof.value.screenshots[0];
    if (field === 'theme') shot.theme = 'dark';
    if (field === 'scale') shot.scale = 125;
    if (field === 'viewport') shot.width = 1440;
    if (field === 'native') shot.source = 'mock';
    if (field === 'target') shot.target_identity = 'sha256:stale';
    if (field === 'hash') shot.sha256 = 'sha256:stale';
    if (field === 'state') shot.states = ['normal'];
    write(proof.record.compliance.path, proof.value);
    proof.record.compliance.sha256 = hash(proof.record.compliance.path); write(proof.path, proof.record);
    expect(gate(data, 'compliance', { stage: 'S' }).result).toBe('FAIL');
  });
  it.each(['Toast', 'RecoveryAction', '§6.7 Feedback', 'A-persistent', 'B-persistent', 'final-persistent'])
  ('rejects weakened two-level scope metadata: %s', missing => {
    const data = setup();
    if (missing === 'Toast' || missing === 'RecoveryAction') data.ui_stages[0].ui_scope.components =
      data.ui_stages[0].ui_scope.components.filter(value => value !== missing);
    if (missing === '§6.7 Feedback') data.ui_stages[0].ui_scope.rules = data.ui_stages[0].ui_scope.rules.filter(value => value !== missing);
    if (missing === 'A-persistent') data.ui_stages[1].ui_scope.states = data.ui_stages[1].ui_scope.states.filter(value => value !== 'toast-persistent');
    if (missing === 'B-persistent') data.ui_stages[2].ui_scope.states = data.ui_stages[2].ui_scope.states.filter(value => value !== 'toast-persistent');
    if (missing === 'final-persistent') data.ui_scope.states = data.ui_scope.states.filter(value => value !== 'toast-persistent');
    expect(gate(data, 'design').result).toBe('FAIL');
  });
  it('rejects generic, standalone and external reuse of a stage reference certificate', () => {
    const data = setup(); const proof = referenceProof(data, 'S');
    expect(evaluateUiGate({ root, artifact: proof.path, phase: 'certification' }).result).toBe('FAIL');
    const consumer = metadata('Proxies', 'TASK-200'); consumer.ui_contract.version = '0.5';
    consumer.ui_scope.golden_pages = [{ page: 'Subscriptions', certification_ref: proof.path }]; write('design.json', consumer);
    expect(gate(consumer, 'readiness', { design: 'design.json' }).result).toBe('FAIL');
  });
  it('rejects missing record-level verification metadata even when the envelope claims reference level', () => {
    const data = setup(); const proof = referenceProof(data, 'S');
    delete proof.value.verification_level; write(proof.record.compliance.path, proof.value);
    proof.record.compliance.sha256 = hash(proof.record.compliance.path); write(proof.path, proof.record);
    expect(gate(data, 'delivery', { stage: 'S' }).result).toBe('FAIL');
  });
  it.each(['task_id', 'stage_id', 'contract', 'contract_version'])('rejects a stage review with wrong %s', field => {
    const data = setup(); const proof = referenceProof(data, 'S', false);
    const review = JSON.parse(readFileSync(join(root, proof.value.review.evidence_ref), 'utf8'));
    review[field] = 'wrong'; write(proof.value.review.evidence_ref, review);
    expect(gate(data, 'compliance', { stage: 'S' }).result).toBe('FAIL');
  });
  it('keeps A/B locked for missing, stale, transitive, or non-Human prerequisites', () => {
    const noHuman = setup(); referenceProof(noHuman, 'S', false); referenceProof(noHuman, 'A');
    expect(gate(noHuman, 'delivery', { stage: 'A' }).result).toBe('FAIL');
    const data = setup(); referenceProof(data, 'S'); referenceProof(data, 'A'); referenceProof(data, 'B');
    rmSync(join(root, data.ui_stages[1].evidence_ref));
    expect(gate(data, 'delivery', { stage: 'B' }).result).toBe('FAIL');
    referenceProof(data, 'A'); referenceProof(data, 'B');
    const sPath = data.ui_stages[0].evidence_ref; const changed = JSON.parse(readFileSync(join(root, sPath), 'utf8'));
    changed.note = 'stale transitive reference'; write(sPath, changed);
    expect(gate(data, 'delivery', { stage: 'B' }).result).toBe('FAIL');
  });
  it('passes final compliance with current references and 30 cells, then waits for exact final Human', () => {
    const data = setup(); allReferenceProofs(data); const final = finalProof(data);
    expect(gate(data, 'compliance', { targetIdentity: final.target }).result).toBe('PASS');
    expect(gate(data, 'delivery', { targetIdentity: final.target }).result).toBe('FAIL');
    finalProof(data, { approved: true });
    expect(gate(data, 'delivery', { targetIdentity: final.target }).result).toBe('PASS');
  });
  it.each(['task_id', 'stage_id', 'contract', 'contract_version'])('rejects a final review with wrong %s', field => {
    const data = setup(); allReferenceProofs(data); const final = finalProof(data);
    const review = JSON.parse(readFileSync(join(root, final.value.review.evidence_ref), 'utf8'));
    review[field] = 'wrong'; write(final.value.review.evidence_ref, review);
    final.value.review.sha256 = hash(final.value.review.evidence_ref); write(final.compliancePath, final.value);
    expect(gate(data, 'compliance', { targetIdentity: final.target }).result).toBe('FAIL');
  });
  it('accepts a complete unique final manifest in its own stored order and hashes that order', () => {
    const data = setup(); allReferenceProofs(data); const final = finalProof(data, { reverseManifest: true });
    expect(final.value.manifest.map(item => item.path)).toEqual([...data.ui_scope.source_paths].reverse());
    expect(gate(data, 'compliance', { targetIdentity: final.target }).result).toBe('PASS');
  });
  it.each(finalCells)('rejects a missing final normal matrix cell $page/$theme/$width/$height/$scale', cell => {
    const data = setup(); allReferenceProofs(data); const final = finalProof(data);
    final.value.screenshots = final.value.screenshots.filter(shot => !Object.entries(cell).every(([key, value]) => shot[key] === value));
    write(final.compliancePath, final.value);
    expect(gate(data, 'compliance', { targetIdentity: final.target }).result).toBe('FAIL');
  });
  it.each(['five-mixed', 'references-only', 'two-pages'])('rejects an incomplete old-style final screenshot set: %s', mode => {
    const data = setup(); const references = allReferenceProofs(data); const final = finalProof(data);
    if (mode === 'five-mixed') final.value.screenshots = final.value.screenshots.slice(0, 5);
    if (mode === 'references-only') final.value.screenshots = references.flatMap(proof => proof.value.screenshots);
    if (mode === 'two-pages') final.value.screenshots = final.value.screenshots.filter(shot => shot.page !== 'Routing');
    write(final.compliancePath, final.value);
    expect(gate(data, 'compliance', { targetIdentity: final.target }).result).toBe('FAIL');
  });
  it.each(['missing', 'duplicate', 'foreign', 'stale', 'stage-mismatch', 'identity'])('rejects invalid final source binding: %s', mode => {
    const data = setup(); allReferenceProofs(data); const final = finalProof(data);
    if (mode === 'missing') final.value.manifest.pop();
    if (mode === 'duplicate') final.value.manifest.push(final.value.manifest[0]);
    if (mode === 'foreign') final.value.manifest[0] = { path: 'src/foreign.tsx', sha256: 'sha256:none' };
    if (mode === 'stale') write(final.value.manifest[0].path, 'changed source bytes');
    if (mode === 'stage-mismatch') write('src/Subscriptions.tsx', 'changed after reference');
    if (mode === 'identity') final.value.target_identity = 'sha256:wrong';
    write(final.compliancePath, final.value);
    expect(gate(data, 'compliance', { targetIdentity: final.target }).result).toBe('FAIL');
  });
  it.each(['missing', 'wrong-path', 'wrong-hash', 'review-list'])('rejects invalid final stage reference binding: %s', mode => {
    const data = setup(); allReferenceProofs(data); const final = finalProof(data);
    if (mode === 'missing') final.value.stage_references.pop();
    if (mode === 'wrong-path') final.value.stage_references[0].path = data.ui_stages[1].evidence_ref;
    if (mode === 'wrong-hash') final.value.stage_references[0].sha256 = 'sha256:wrong';
    if (mode === 'review-list') {
      const review = JSON.parse(readFileSync(join(root, final.value.review.evidence_ref), 'utf8'));
      review.stage_references.pop(); write(final.value.review.evidence_ref, review);
      final.value.review.sha256 = hash(final.value.review.evidence_ref);
    }
    write(final.compliancePath, final.value);
    expect(gate(data, 'compliance', { targetIdentity: final.target }).result).toBe('FAIL');
  });
  it('binds final review bytes to compliance and final compliance bytes to Human', () => {
    const data = setup(); allReferenceProofs(data); const final = finalProof(data, { approved: true });
    const review = JSON.parse(readFileSync(join(root, final.value.review.evidence_ref), 'utf8'));
    review.summary = 'changed after compliance'; write(final.value.review.evidence_ref, review);
    expect(gate(data, 'delivery', { targetIdentity: final.target }).result).toBe('FAIL');
    finalProof(data, { approved: true });
    const complianceValue = JSON.parse(readFileSync(join(root, final.compliancePath), 'utf8'));
    complianceValue.note = 'changed after Human'; write(final.compliancePath, complianceValue);
    expect(gate(data, 'delivery', { targetIdentity: final.target }).result).toBe('FAIL');
  });
  it.each(['missing-ref', 'unknown-field', 'outside', 'mismatch', 'zero', 'alias'])('rejects invalid final locator metadata: %s', mode => {
    const data = setup();
    if (mode === 'missing-ref') delete data.ui_verification.final_evidence.human_approval_ref;
    if (mode === 'unknown-field') data.ui_verification.final_evidence.latest = true;
    if (mode === 'outside') data.ui_verification.final_evidence.compliance_ref = '../compliance-001.json';
    if (mode === 'mismatch') data.ui_verification.final_evidence.human_approval_ref = '.sdlc/evidence/TASK-019/final-visual/human-approval-002.json';
    if (mode === 'zero') { data.ui_verification.final_evidence.compliance_ref = '.sdlc/evidence/TASK-019/final-visual/compliance-000.json';
      data.ui_verification.final_evidence.human_approval_ref = '.sdlc/evidence/TASK-019/final-visual/human-approval-000.json'; }
    if (mode === 'alias') data.ui_verification.final_evidence.compliance_ref = '.sdlc/evidence/TASK-019/final-visual/latest.json';
    expect(gate(data, 'design').result).toBe('FAIL');
  });
  it('does not fall back to legacy fixed evidence or discover an unadopted newer attempt', () => {
    const data = setup(); allReferenceProofs(data); const final = finalProof(data);
    renameSync(join(root, final.compliancePath), join(root, '.sdlc/evidence/TASK-019/ui-contract-compliance.json'));
    write('.sdlc/evidence/TASK-019/final-visual/compliance-002.json', final.value);
    expect(gate(data, 'compliance', { targetIdentity: final.target }).result).toBe('FAIL');
  });
  it('rejects symlink indirection at the exact adopted final evidence path', () => {
    const data = setup(); allReferenceProofs(data); const final = finalProof(data);
    const selectedDir = join(root, '.sdlc/evidence/TASK-019/final-visual');
    const realDir = join(root, '.sdlc/evidence/TASK-019/final-real');
    renameSync(selectedDir, realDir); symlinkSync(realDir, selectedDir, 'junction');
    expect(gate(data, 'compliance', { targetIdentity: final.target }).result).toBe('FAIL');
  });
  it.each(['missing-model', 'unknown-model', 'wrong-task', 'wrong-stage', 'shared'])('rejects illegal two-level declarations: %s', mode => {
    const data = setup();
    if (mode === 'missing-model') delete data.ui_verification.model;
    if (mode === 'unknown-model') data.ui_verification.model = 'other';
    if (mode === 'wrong-task') data.task_id = 'TASK-020';
    if (mode === 'wrong-stage') data.ui_stages[2].id = 'C';
    if (mode === 'shared') data.ui_scope.delivery_mode = 'cross_cutting_shared';
    expect(gate(data, 'design').result).toBe('FAIL');
  });
  it('rejects no-stage checkpoint and exact final Human omissions or stale screenshot lists', () => {
    const data = setup(); expect(gate(data, 'checkpoint').result).toBe('FAIL');
    allReferenceProofs(data); const final = finalProof(data, { approved: true });
    const humanPath = data.ui_verification.final_evidence.human_approval_ref;
    const human = JSON.parse(readFileSync(join(root, humanPath), 'utf8'));
    human.reviewed_screenshots.pop(); write(humanPath, human);
    expect(gate(data, 'delivery', { targetIdentity: final.target }).result).toBe('FAIL');
  });
});
