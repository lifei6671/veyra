import { lstatSync, readFileSync, realpathSync, statSync } from 'node:fs';
import { resolve, relative, isAbsolute } from 'node:path';
import { createHash } from 'node:crypto';
import { isDeepStrictEqual } from 'node:util';

export const CONTRACT = 'docs/ui/veyra-ui-spec.md';
const list = value => Array.isArray(value) && value.length > 0 && value.every(x => typeof x === 'string' && x.trim());
const contains = (actual, required) => Array.isArray(actual) && required.every(x => actual.includes(x));
const digest = bytes => 'sha256:' + createHash('sha256').update(bytes).digest('hex');
const shared = task => task.ui_scope?.delivery_mode === 'cross_cutting_shared';
const TWO_LEVEL_MODEL = 'stage_reference_then_final_matrix';
const REFERENCE_LEVEL = 'stage_reference';
const FINAL_LEVEL = 'task_final_matrix';
const FINAL_ENVIRONMENTS = [[1280, 720, 100], [1280, 720, 125], [1280, 720, 150], [1440, 900, 100], [1440, 900, 125]];
// Object key order is not materialization semantics; array order remains meaningful.
const ordered = value => Array.isArray(value) ? value.map(ordered)
  : value && typeof value === 'object'
    ? Object.fromEntries(Object.keys(value).sort().map(key => [key, ordered(value[key])])) : value;

// Read-only: never produces approvals or mutates Task/Gate state.
export function evaluateUiGate({ root, artifact, design, phase, stage, targetIdentity }) {
  const findings = [];
  const fail = message => { throw new Error(message); };
  function bytes(path) {
    if (typeof path !== 'string' || !path) fail('Missing evidence/document path');
    const full = resolve(root, path);
    if (isAbsolute(path) || relative(resolve(root), full).startsWith('..')) fail('Reference must stay inside repository');
    if (!statSync(full).isFile()) fail('Reference is not a file: ' + path);
    return readFileSync(full);
  }
  function immutableBytes(path) {
    const full = resolve(root, path);
    if (lstatSync(full).isSymbolicLink() || realpathSync(full) !== full) fail('Immutable evidence reference cannot use symlink indirection');
    return bytes(path);
  }
  const json = path => JSON.parse(bytes(path).toString());
  function load(path) {
    const text = bytes(path).toString();
    if (path.endsWith('.json')) return { data: JSON.parse(text), scopeText: text };
    const blocks = [...text.matchAll(/^## UI Delivery Metadata\r?\n\s*```json\r?\n([\s\S]*?)\r?\n```/gm)];
    if (blocks.length > 1) fail('Duplicate UI Delivery Metadata');
    const data = blocks.length ? JSON.parse(blocks[0][1]) : {};
    // Only positive Scope, not deny lists, history, or incidental file extensions.
    const scopeText = text.match(/^## Scope\r?\n([\s\S]*?)(?=^### deny|^## |$(?![\s\S]))/m)?.[1] ??
      [...text.matchAll(/^## .*?(?:UI|视觉|页面).*\r?\n([\s\S]*?)(?=^## |$(?![\s\S]))/gm)].map(match => match[1]).join('\n');
    return { data, scopeText, text };
  }
  function impacting(doc, visited = new Set()) {
    const { deny, ...positive } = doc.data.scope ?? {};
    const scope = doc.data.scope ? JSON.stringify(positive) : doc.scopeText;
    if (doc.data.ui_impact?.required === true || !!doc.data.ui_contract ||
      /\b(AppShell|Sidebar|PageHeader|Page|Dialog|Toast|Notice|CSS|styles?|tokens?|dense list|visual component|interaction state|user-visible layout)\b|页面|视觉|布局|交互状态/i.test(scope)) return true;
    // Existing Design Candidates are manifest envelopes; inspect their actual artifacts.
    for (const entry of doc.data.manifest ?? []) {
      if (visited.has(entry.path)) continue;
      visited.add(entry.path);
      if (impacting(load(entry.path), visited)) return true;
    }
    return false;
  }
  function declaration(doc, version) {
    const data = doc.data;
    const contract = data.ui_contract;
    if (!contract || contract.document !== CONTRACT || contract.version !== version || contract.required !== true)
      fail('UI Contract missing, wrong path/version, or not required');
    bytes(contract.document);
    const scope = data.ui_scope;
    for (const key of ['pages', 'components', 'states', 'rules'])
      if (!list(scope?.[key])) fail('Missing UI scope: ' + key);
    const pages = ['Overview', 'Subscriptions', 'Proxies', 'Routing', 'Settings', 'Connections', 'Logs'];
    if (scope.pages.some(page => !pages.includes(page))) fail('UI scope pages must use canonical Contract page names');
    if (data.ui_impact?.required !== true) fail('UI-impact assessment must declare required: true');
    if (!data.ui_impact.reason?.trim()) fail('UI-impact assessment needs scope-based reason');
    for (const state of ['normal', 'hover', 'selected', 'disabled', 'loading', 'empty', 'error', 'busy', 'focus', 'success']) {
      if (!scope.states.includes(state) && !scope.state_exclusions?.[state]?.trim()) fail('Undeclared visual state: ' + state);
    }
    if (data.visual_gate?.required !== true || data.visual_gate?.human_approval !== true)
      fail('Visual acceptance and human approval must be required');
    if (scope.delivery_mode !== undefined && !['page_migration', 'cross_cutting_shared'].includes(scope.delivery_mode))
      fail('Unknown UI delivery mode');
    if (shared(data)) {
      if (data.ui_stages !== undefined || scope.golden_pages !== undefined || data.produces_golden_page !== undefined || scope.produces_golden_page !== undefined)
        fail('Shared delivery cannot declare Golden Pages or UI stages');
      if (!list(scope.shared_components) || !contains(scope.components, scope.shared_components) ||
          !list(scope.source_paths) || !list(scope.verification_pages) || !contains(scope.pages, scope.verification_pages))
        fail('Shared components, source paths and representative verification pages required');
      // Classification is independently reviewed, not a producer-controlled bypass flag.
      const assessment = json(scope.shared_review);
      const { shared_review, ...reviewedScope } = scope;
      const scopeIdentity = digest(JSON.stringify(ordered({ task_id: data.task_id, scope: data.scope, ui_scope: reviewedScope })));
      if (assessment.result !== 'PASS' || assessment.delivery_mode !== 'cross_cutting_shared' ||
          assessment.page_migration !== false || assessment.scope_identity !== scopeIdentity ||
          !assessment.reviewer?.trim() || !assessment.produced_by?.trim() || assessment.reviewer === assessment.produced_by ||
          !assessment.summary?.trim() || !assessment.timestamp || !Number.isFinite(Date.parse(assessment.timestamp)))
        fail('Independent shared-only scope classification missing/stale/rejected');
    }
    const verification = data.ui_verification;
    if (verification !== undefined) {
      if (verification?.model !== TWO_LEVEL_MODEL) fail('Unknown UI verification model');
      if (version !== '0.5' || data.task_id !== 'TASK-019' || shared(data))
        fail('Two-level verification model is only valid for TASK-019 under Contract0.5');
      if (!list(scope.source_paths) || new Set(scope.source_paths).size !== scope.source_paths.length)
        fail('TASK-019 final source paths must be complete and unique');
      if (!isDeepStrictEqual(Object.keys(verification).sort(), ['final_evidence', 'model']))
        fail('Two-level verification declaration has missing/unknown fields');
      const final = verification.final_evidence;
      if (!final || !isDeepStrictEqual(Object.keys(final).sort(), ['compliance_ref', 'human_approval_ref']))
        fail('Final evidence locator has missing/unknown fields');
      const compliance = /^\.sdlc\/evidence\/TASK-019\/final-visual\/compliance-(\d{3})\.json$/.exec(final.compliance_ref ?? '');
      const human = /^\.sdlc\/evidence\/TASK-019\/final-visual\/human-approval-(\d{3})\.json$/.exec(final.human_approval_ref ?? '');
      if (!compliance || !human || compliance[1] === '000' || compliance[1] !== human[1])
        fail('Final evidence locator must select one positive matching versioned attempt');
    }
    return data;
  }
  function evidence(task, version, identity, refs, requireHuman = true, context = {}) {
    if (!refs && !/^TASK-\d+$/.test(task.task_id)) fail('Invalid evidence task_id');
    const base = '.sdlc/evidence/' + task.task_id + '/';
    const compliancePath = context.level === FINAL_LEVEL
      ? task.ui_verification.final_evidence.compliance_ref
      : refs?.compliance.path ?? base + 'ui-contract-compliance.json';
    const readEvidence = context.level === FINAL_LEVEL ? immutableBytes : bytes;
    const compliance = JSON.parse(readEvidence(compliancePath).toString());
    if (context.level && (compliance.verification_level !== context.level || compliance.verification_model !== TWO_LEVEL_MODEL))
      fail('Compliance verification level/model mismatch');
    if (refs && compliance.certification_kind !== refs.kind) fail('Compliance certification kind mismatch');
    if (refs && !isDeepStrictEqual(compliance.prerequisites, refs.prerequisites)) fail('Compliance prerequisite binding mismatch');
    const subjectMatches = record => refs?.subject.certification_id
      ? record.certification_id === refs.subject.certification_id
      : record.task_id === task.task_id && (refs ? record.stage_id === refs.subject.stage_id : record.stage_id === undefined);
    if (refs && digest(bytes(compliancePath)) !== refs.compliance.sha256) fail('Compliance hash changed');
    if (!identity || compliance.target_identity !== identity) fail('Compliance target identity missing/stale');
    if (!subjectMatches(compliance) || compliance.contract !== CONTRACT || compliance.contract_version !== version || compliance.result !== 'PASS')
      fail('UI compliance missing/mismatched/FAIL');
    if (!Array.isArray(compliance.findings) || compliance.findings.length) fail('Unresolved UI compliance findings');
    for (const [key, checked] of [['pages', 'checked_pages'], ['components', 'checked_components'], ['states', 'checked_states'], ['rules', 'checked_rules']])
      if (!contains(compliance[checked], task.ui_scope[key])) fail('Compliance coverage missing: ' + key);
    for (const command of ['pnpm lint', 'pnpm test', 'pnpm build']) {
      const check = compliance.verification?.find(item => item.command === command);
      if (!check || check.exit_code !== 0 || check.result !== 'PASS' || check.target_identity !== identity)
        fail('Missing successful current frontend verification: ' + command);
      const record = json(check.evidence_ref);
      if (record.result !== 'PASS' || record.exit_code !== 0 || record.target_identity !== identity ||
          !(record.command_or_method === command || record.command_or_method?.startsWith(command + ' ')) ||
          !record.observer?.trim() || !record.summary?.trim() || !record.timestamp || !Number.isFinite(Date.parse(record.timestamp)))
        fail('Frontend verification reference does not prove the declared check: ' + command);
    }
    if (compliance.review?.result !== 'PASS' || !compliance.review.reviewer || compliance.review.reviewer === compliance.produced_by)
      fail('Independent CSS/token/composition review required');
    const review = json(compliance.review.evidence_ref);
    if (context.level && (review.verification_level !== context.level || review.verification_model !== TWO_LEVEL_MODEL))
      fail('Independent review verification level/model mismatch');
    if (context.level && (review.task_id !== task.task_id || review.contract !== CONTRACT || review.contract_version !== version ||
        (context.level === REFERENCE_LEVEL ? review.stage_id !== refs.subject.stage_id : review.stage_id !== undefined)))
      fail('Independent review subject/Contract mismatch');
    if (context.level === FINAL_LEVEL && compliance.review.sha256 !== digest(bytes(compliance.review.evidence_ref)))
      fail('Final compliance independent review hash missing/stale');
    if (refs && review.certification_kind !== refs.kind) fail('Independent review certification kind mismatch');
    if (refs && !isDeepStrictEqual(review.prerequisites, refs.prerequisites)) fail('Independent review prerequisite binding mismatch');
    if (review.result !== 'PASS' || review.target_identity !== identity || review.reviewer !== compliance.review.reviewer ||
        !review.command_or_method?.trim() || !review.summary?.trim() || !review.timestamp || !Number.isFinite(Date.parse(review.timestamp)))
      fail('Independent review evidence missing/mismatched');
    for (const key of ['pages', 'components', 'states', 'rules'])
      if (!contains(review.coverage?.[key], task.ui_scope[key])) fail('Independent review coverage missing: ' + key);
    if (task.ui_scope.source_paths && !contains(review.coverage?.source_paths, task.ui_scope.source_paths))
      fail('Independent review source coverage missing');
    if (!Array.isArray(compliance.screenshots) || !compliance.screenshots.length) fail('Screenshots required');
    for (const shot of compliance.screenshots) {
      if (shot.source !== 'tauri-windows' || shot.target_identity !== identity || digest(bytes(shot.path)) !== shot.sha256)
        fail('Screenshot missing, changed, non-native, or stale');
    }
    const verificationPages = shared(task) ? task.ui_scope.verification_pages : task.ui_scope.pages;
    const shotsForVerification = compliance.screenshots.filter(shot => verificationPages.includes(shot.page));
    if (context.level === REFERENCE_LEVEL) {
      if (compliance.screenshots.some(shot => shot.page !== task.ui_scope.pages[0] || shot.theme !== 'light' ||
          shot.width !== 1280 || shot.height !== 720 || shot.scale !== 150))
        fail('Stage reference screenshots must use the exact reference page/environment');
      for (const state of task.ui_scope.states)
        if (!shotsForVerification.some(shot => shot.states?.includes(state))) fail('Missing state screenshot: ' + state);
    } else if (context.level === FINAL_LEVEL) {
      for (const page of task.ui_scope.pages)
        for (const theme of ['light', 'dark'])
          for (const [width, height, scale] of FINAL_ENVIRONMENTS)
            if (!shotsForVerification.some(shot => shot.page === page && shot.theme === theme && shot.width === width &&
                shot.height === height && shot.scale === scale && shot.states?.includes('normal')))
              fail('Missing final normal matrix cell: ' + [page, theme, width, height, scale].join('/'));
    } else {
      for (const state of task.ui_scope.states)
        if (!shotsForVerification.some(shot => shot.states?.includes(state)))
          fail('Missing state screenshot: ' + state);
    }
    if (shared(task)) {
      for (const page of verificationPages)
        if (!shotsForVerification.some(shot => shot.page === page)) fail('Missing shared integration screenshot: ' + page);
      for (const component of task.ui_scope.shared_components) {
        const componentShots = shotsForVerification.filter(shot => shot.components?.includes(component));
        for (const page of verificationPages)
          if (!componentShots.some(shot => shot.page === page)) fail('Missing shared component integration screenshot: ' + component + '/' + page);
        for (const state of task.ui_scope.states)
          if (!componentShots.some(shot => shot.states?.includes(state))) fail('Missing shared component state screenshot: ' + component + '/' + state);
        for (const theme of ['light', 'dark'])
          for (const scale of [100, 125, 150])
            if (!componentShots.some(shot => shot.theme === theme && shot.scale === scale &&
                Number.isFinite(shot.width) && shot.width > 0 && Number.isFinite(shot.height) && shot.height > 0 && shot.states?.includes('normal')))
              fail('Missing shared theme/DPI screenshot: ' + component + '/' + theme + '/' + scale);
      }
    } else if (!context.level) for (const page of task.ui_scope.pages) {
      const shots = compliance.screenshots.filter(shot => shot.page === page);
      for (const theme of ['light', 'dark'])
        if (!shots.some(shot => shot.theme === theme && shot.states?.includes('normal'))) fail('Missing normal theme screenshot: ' + page + '/' + theme);
      for (const [width, height, scale] of [[1280, 720, 100], [1440, 900, 100], [1280, 720, 125], [1440, 900, 125], [1280, 720, 150]])
        if (!shots.some(shot => shot.width === width && shot.height === height && shot.scale === scale)) fail('Missing viewport/DPI screenshot: ' + page);
    }
    if (!requireHuman) return compliance;
    const approvalPath = context.level === FINAL_LEVEL
      ? task.ui_verification.final_evidence.human_approval_ref
      : refs?.human_approval?.path ?? base + 'ui-human-approval.json';
    const approval = context.level === FINAL_LEVEL ? JSON.parse(immutableBytes(approvalPath).toString()) : json(approvalPath);
    if (context.level && (approval.verification_level !== context.level || approval.verification_model !== TWO_LEVEL_MODEL))
      fail('Human approval verification level/model mismatch');
    if (refs && digest(bytes(approvalPath)) !== refs.human_approval?.sha256) fail('Human approval reference missing/changed');
    if (!subjectMatches(approval) || approval.contract_version !== version || approval.contract !== CONTRACT ||
        approval.target_identity !== identity || approval.approved_by !== 'human' || approval.decision !== 'APPROVED' ||
        approval.recorded_by !== 'human' || !approval.timestamp || !Number.isFinite(Date.parse(approval.timestamp)))
      fail('WAITING_HUMAN_VISUAL_ACCEPTANCE: explicit human-owned approval required');
    if (approval.compliance_sha256 !== digest(readEvidence(compliancePath)) ||
        (context.level === FINAL_LEVEL
          ? !isDeepStrictEqual(approval.reviewed_screenshots, compliance.screenshots.map(shot => shot.path))
          : !contains(approval.reviewed_screenshots, compliance.screenshots.map(shot => shot.path))))
      fail('Human approval does not cover current compliance/screenshots');
    return compliance;
  }
  // Page certification is an Evidence envelope, independent of historical Feature Tasks.
  function pageEvidence(path, version, expected, requireHuman = true, authenticated) {
    const record = json(path);
    if (shared(record)) fail('Shared delivery cannot certify a Golden Page');
    if (!['golden_page_certification', 'ui_stage_acceptance'].includes(record.kind) ||
        record.page !== expected.page || record.contract !== CONTRACT || record.contract_version !== version)
      fail('Page certification identity/Contract mismatch');
    if (expected.golden && record.kind !== 'golden_page_certification') fail('Golden Page certification required');
    if (expected.task_id && !expected.golden && record.kind !== 'ui_stage_acceptance')
      fail('Stage is not an approved Golden Page producer');
    if (expected.task_id && (record.task_id !== expected.task_id || record.stage_id !== expected.stage_id))
      fail('Intra-task stage certification subject mismatch');
    const isStage = typeof record.task_id === 'string' && typeof record.stage_id === 'string';
    if (!(isStage && /^TASK-\d+$/.test(record.task_id) && /^[A-Za-z0-9_-]+$/.test(record.stage_id)) &&
        !(typeof record.certification_id === 'string' && record.certification_id.trim())) fail('Certification subject required');
    if (isStage && record.certification_id !== undefined) fail('Certification subject is ambiguous');
    if (!isStage && (record.task_id !== undefined || record.stage_id !== undefined)) fail('Incomplete certification stage subject');
    const claimsReference = record.verification_level === REFERENCE_LEVEL || record.verification_model === TWO_LEVEL_MODEL;
    const isReference = authenticated?.task.ui_verification?.model === TWO_LEVEL_MODEL;
    if (claimsReference && !isReference) fail('Stage reference evidence requires authenticated same-Task stage context');
    if (isReference && (authenticated.task.task_id !== record.task_id || authenticated.stage.id !== record.stage_id ||
        record.verification_level !== REFERENCE_LEVEL || record.verification_model !== TWO_LEVEL_MODEL))
      fail('Stage reference evidence requires authenticated same-Task stage context');
    const data = declaration({ data: {
      task_id: record.task_id, ui_impact: { required: true, reason: 'Page certification' },
      ui_contract: { document: record.contract, version: record.contract_version, required: true },
      ui_scope: record.ui_scope, visual_gate: { required: true, human_approval: true },
    } }, version);
    if (!isDeepStrictEqual(data.ui_scope.pages, [record.page]) ||
        (expected.ui_scope && !isDeepStrictEqual(expected.ui_scope, data.ui_scope))) fail('Stage/page certification scope mismatch');
    if (!list(data.ui_scope.source_paths) || !Array.isArray(record.manifest) || !record.manifest.length ||
        !/^[a-f0-9]{40}$/.test(record.git_identity ?? '')) fail('Certification source/Git identity required');
    const paths = record.manifest.map(item => item.path);
    if (new Set(paths).size !== paths.length || paths.length !== data.ui_scope.source_paths.length ||
        !contains(paths, data.ui_scope.source_paths)) fail('Certification source manifest incomplete');
    for (const item of record.manifest)
      if (digest(bytes(item.path)) !== item.sha256) fail('Page certification source is stale: ' + item.path);
    // Identity is page-scoped so unrelated later stage changes do not stale an approval.
    if (record.target_identity !== digest(JSON.stringify({ git_identity: record.git_identity, manifest: record.manifest })))
      fail('Certification delivery identity mismatch');
    if (expected.targetIdentity && record.target_identity !== expected.targetIdentity) fail('Unexpected certification target');
    const needed = prerequisites([record.page]);
    if (!Array.isArray(record.prerequisites) || record.prerequisites.length !== needed.length ||
        !isDeepStrictEqual(record.prerequisites.map(item => item.page).sort(), [...needed].sort()))
      fail('Certification Golden Page prerequisites incomplete');
    for (const link of record.prerequisites) {
      if (digest(bytes(link.certification_ref)) !== link.sha256) fail('Prerequisite certification hash mismatch');
      if (isReference) {
        const producer = authenticated.stages.find(item => item.produces_golden_page === link.page);
        if (!producer) fail('Stage reference prerequisite producer missing');
        pageEvidence(link.certification_ref, version, { page: link.page, golden: true, ui_scope: producer.ui_scope,
          task_id: record.task_id, stage_id: producer.id }, true,
          { task: authenticated.task, stage: producer, stages: authenticated.stages });
      } else pageEvidence(link.certification_ref, version, { page: link.page, golden: true });
    }
    evidence(data, version, record.target_identity, { ...record, subject: record }, requireHuman,
      isReference ? { level: REFERENCE_LEVEL } : {});
    return record;
  }
  const prerequisites = pages => [...new Set(pages.flatMap(page =>
    page === 'Proxies' ? ['Subscriptions'] : ['Routing', 'Connections', 'Logs'].includes(page) ? ['Proxies'] : []))];
  function stagesFor(task, version) {
    if (shared(task)) return [];
    const stages = task.ui_stages ?? [];
    if (!Array.isArray(stages)) fail('ui_stages must be an ordered array');
    const ids = new Set();
    for (const item of stages) {
      if (!/^[A-Za-z0-9_-]+$/.test(item.id ?? '') || ids.has(item.id) || !item.evidence_ref) fail('Unique stage ID and evidence_ref required');
      ids.add(item.id);
      declaration({ data: { ...task, ui_scope: item.ui_scope } }, version);
      if (item.ui_scope.pages.length !== 1 || !list(item.ui_scope.source_paths)) fail('Stage must declare one page and its source_paths');
      if (item.produces_golden_page && item.produces_golden_page !== item.ui_scope.pages[0]) fail('Stage Golden Page producer mismatch');
      const needed = prerequisites(item.ui_scope.pages);
      if (!Array.isArray(item.requires_golden_pages) || !isDeepStrictEqual([...item.requires_golden_pages].sort(), [...needed].sort()))
        fail('Stage prerequisites must match Contract');
    }
    if (stages.length) {
      for (const key of ['pages', 'components', 'states', 'rules']) {
        const all = stages.flatMap(item => item.ui_scope[key]);
        if (!contains(all, task.ui_scope[key]) || !contains(task.ui_scope[key], all) ||
            (key === 'pages' && new Set(all).size !== all.length)) fail('Stages must cover Task UI scope exactly: ' + key);
      }
      for (const [index, item] of stages.entries())
        for (const page of item.requires_golden_pages) {
          const producer = stages.findIndex(other => other.produces_golden_page === page);
          if (producer >= index) fail('Golden Page stage prerequisite must be produced earlier');
          if (producer < 0 && stages.some(other => other.ui_scope.pages.includes(page))) fail('Intra-task Golden Page producer declaration missing');
        }
    } else if (prerequisites(task.ui_scope.pages).some(page => task.ui_scope.pages.includes(page))) {
      fail('Intra-task Golden Page dependency requires explicit ui_stages');
    }
    if (task.ui_verification?.model === TWO_LEVEL_MODEL) {
      const shape = stages.map(item => ({ id: item.id, page: item.ui_scope.pages[0], requires: item.requires_golden_pages,
        produces: item.produces_golden_page }));
      if (!isDeepStrictEqual(shape, [
        { id: 'S', page: 'Subscriptions', requires: [], produces: 'Subscriptions' },
        { id: 'A', page: 'Proxies', requires: ['Subscriptions'], produces: 'Proxies' },
        { id: 'B', page: 'Routing', requires: ['Proxies'], produces: undefined },
      ])) fail('TASK-019 two-level stage shape must be exact');
      for (const item of stages) {
        for (const key of ['pages', 'components', 'states', 'rules', 'source_paths'])
          if (new Set(item.ui_scope[key]).size !== item.ui_scope[key].length) fail('TASK-019 stage scope entries must be unique: ' + key);
        if (!contains(item.ui_scope.components, ['Toast', 'RecoveryAction']) || !item.ui_scope.rules.includes('§6.7 Feedback'))
          fail('TASK-019 reference scope is missing required shared feedback metadata');
        if ((item.id === 'S') === item.ui_scope.states.includes('toast-persistent'))
          fail('Only Stage S may exclude toast-persistent');
      }
      if (!task.ui_scope.states.includes('toast-persistent')) fail('TASK-019 final scope must retain toast-persistent');
    }
    return stages;
  }

  function finalBindings(task, stages, version) {
    const records = stages.map(item => pageEvidence(item.evidence_ref, version, {
      page: item.ui_scope.pages[0], ui_scope: item.ui_scope, task_id: task.task_id, stage_id: item.id,
      golden: !!item.produces_golden_page,
    }, true, { task, stage: item, stages }));
    const compliancePath = task.ui_verification.final_evidence.compliance_ref;
    const compliance = JSON.parse(immutableBytes(compliancePath).toString());
    const sourcePaths = [...new Set([...(task.ui_scope.source_paths ?? []), ...stages.flatMap(item => item.ui_scope.source_paths)])];
    const finalPaths = compliance.manifest?.map(item => item.path);
    if (!/^[a-f0-9]{40}$/.test(compliance.git_identity ?? '') || !Array.isArray(compliance.manifest) ||
        new Set(finalPaths).size !== finalPaths.length || finalPaths.length !== sourcePaths.length ||
        !contains(finalPaths, sourcePaths) || !contains(sourcePaths, finalPaths))
      fail('Final source union manifest is incomplete, duplicated, or foreign');
    for (const item of compliance.manifest)
      if (digest(bytes(item.path)) !== item.sha256) fail('Final source union is stale: ' + item.path);
    const finalIdentity = digest(JSON.stringify({ git_identity: compliance.git_identity, manifest: compliance.manifest }));
    if (compliance.target_identity !== finalIdentity) fail('Final source union identity mismatch');
    const byPath = new Map(compliance.manifest.map(item => [item.path, item.sha256]));
    for (const record of records)
      for (const item of record.manifest)
        if (byPath.get(item.path) !== item.sha256) fail('Stage source manifest does not match final source union');
    const references = stages.map((item, index) => ({ stage_id: item.id, path: item.evidence_ref,
      sha256: digest(bytes(item.evidence_ref)), record: records[index] }));
    const publicReferences = references.map(({ record: _record, ...reference }) => reference);
    if (!isDeepStrictEqual(compliance.stage_references, publicReferences)) fail('Final stage reference list missing/stale/mismatched');
    const review = json(compliance.review?.evidence_ref);
    if (!isDeepStrictEqual(review.stage_references, publicReferences)) fail('Final independent review stage references mismatch');
    return { identity: finalIdentity, references: publicReferences };
  }
  function externalPrerequisites(task, stages, version) {
    if (shared(task)) return;
    for (const page of prerequisites(task.ui_scope.pages)) {
      if (stages.some(item => item.produces_golden_page === page)) continue;
      const link = task.ui_scope.golden_pages?.find(item => item.page === page);
      if (!link?.certification_ref) fail('External Golden Page certification required: ' + page);
      const external = pageEvidence(link.certification_ref, version, { page, golden: true });
      if (external.task_id === task.task_id) fail('External prerequisite cannot be produced by the same Task');
    }
  }
  try {
    if (!['design', 'readiness', 'checkpoint', 'compliance', 'certification', 'delivery'].includes(phase)) fail('Unknown UI gate phase');
    const doc = load(artifact);
    const designDoc = design ? load(design) : null;
    const isCertification = ['golden_page_certification', 'ui_stage_acceptance'].includes(doc.data.kind);
    if (phase === 'certification' && !isCertification) fail('Certification phase requires a page certification envelope');
    if (!isCertification && !impacting(doc) && !(designDoc && impacting(designDoc))) return { result: 'PASS', applicable: false, findings };
    const spec = bytes(CONTRACT).toString();
    const version = spec.match(/^Version: "?([\d.]+)"?\r?$/m)?.[1];
    if (!version || !/^Status: APPROVED\r?$/m.test(spec) || !/^Contract: BINDING\r?$/m.test(spec)) fail('Current UI Contract is not APPROVED/BINDING');
    if (isCertification) {
      if (!['compliance', 'certification'].includes(phase)) fail('Certification envelope is not a Task/Design');
      pageEvidence(artifact, version, { page: doc.data.page, targetIdentity, golden: true }, phase === 'certification');
      return { result: 'PASS', applicable: true, scope: 'certification', findings };
    }
    const task = declaration(doc, version);
    const stages = stagesFor(task, version);
    const currentStage = stage ? stages.find(item => item.id === stage) : null;
    if (stage && (!currentStage || !['checkpoint', 'compliance', 'delivery'].includes(phase))) fail('Invalid stage checkpoint/acceptance selector');
    const frontId = doc.text?.match(/^id: (TASK-\d+)\r?$/m)?.[1];
    if (frontId && task.task_id !== frontId) fail('Task metadata ID differs from front matter');
    if (designDoc) {
      const frozen = declaration(designDoc, version);
      if (task.task_id !== frozen.task_id) fail('Task/Design ID mismatch');
      for (const key of ['ui_contract', 'ui_scope', 'visual_gate', 'ui_stages', 'ui_verification'])
        if (!isDeepStrictEqual(task[key], frozen[key])) fail('Materialized Task lost/changed Design UI metadata: ' + key);
    }
    if (phase === 'readiness' && !designDoc) fail('Readiness requires approved Design reference');
    if (['readiness', 'checkpoint', 'compliance', 'delivery'].includes(phase)) externalPrerequisites(task, stages, version);
    if (stages.length && ['checkpoint', 'compliance'].includes(phase) && !currentStage && task.ui_verification?.model !== TWO_LEVEL_MODEL)
      fail('Select the actual UI stage');
    if (stages.length && phase === 'checkpoint' && !currentStage) fail('Select the actual UI stage');
    const stageEvidence = item => pageEvidence(item.evidence_ref, version, {
      page: item.ui_scope.pages[0], ui_scope: item.ui_scope, task_id: task.task_id, stage_id: item.id,
      golden: !!item.produces_golden_page,
    }, true, task.ui_verification?.model === TWO_LEVEL_MODEL ? { task, stage: item, stages } : undefined);
    if (currentStage) {
      for (const page of currentStage.requires_golden_pages) {
        const producer = stages.find(item => item.produces_golden_page === page);
        if (producer) stageEvidence(producer);
      }
      if (phase === 'delivery') stageEvidence(currentStage);
      if (phase === 'compliance') pageEvidence(currentStage.evidence_ref, version, {
        page: currentStage.ui_scope.pages[0], ui_scope: currentStage.ui_scope, task_id: task.task_id,
        stage_id: currentStage.id, golden: !!currentStage.produces_golden_page,
      }, false, task.ui_verification?.model === TWO_LEVEL_MODEL ? { task, stage: currentStage, stages } : undefined);
    } else if (phase === 'delivery') {
      if (task.ui_verification?.model === TWO_LEVEL_MODEL) {
        const final = finalBindings(task, stages, version);
        if (targetIdentity !== final.identity) fail('Unexpected final delivery target');
        evidence(task, version, final.identity, undefined, true, { level: FINAL_LEVEL });
      } else {
        for (const item of stages) stageEvidence(item);
        evidence(task, version, targetIdentity);
      }
    } else if (phase === 'compliance') {
      if (task.ui_verification?.model === TWO_LEVEL_MODEL) {
        const final = finalBindings(task, stages, version);
        if (targetIdentity !== final.identity) fail('Unexpected final compliance target');
        evidence(task, version, final.identity, undefined, false, { level: FINAL_LEVEL });
      } else evidence(task, version, targetIdentity, undefined, false);
    }
    return { result: 'PASS', applicable: true, scope: currentStage ? 'stage' : 'task', findings };
  } catch (error) {
    findings.push(error.message);
    return { result: 'FAIL', applicable: true, findings };
  }
}
