# TASK-018 focus separation amendment005

Status: CANDIDATE — exact Human Change/Design Approval pending.

## Revises only the focus assumption

DCR022 replaces design003's fixed outline-offset:-4px and execution004's inherited reliance on it. Contract0.3 and DCR021 do not change. The prior source implements the frozen offset but native Review proves its separation assumption false. The original38 inventory is preserved; other37 implementations are not reopened.

Before:2px primary outline/-4px was specified and assumed to leave a visible gap.
After:keep2px primary outer active border and normal content surface; focus uses a separate2px primary outline with enough local inset for a visibly distinct surface-colored gap. Try -6px first after approval, then judge actual native rendering. Exact offset is a bounded local implementation detail, not a frozen token or a claim of visual success. No new colors/shadows/wrappers. No geometry-affecting border/padding/layout changes.

## Allowed new delta

Only src/styles.css SubscriptionCard :focus-visible and directly necessary active/focus separation presentation. Existing base Card, active border/background/padding, inactive/active hover, DOM, TSX, keyboard behavior, business state/handler, shared tokens, Sidebar/Proxies/Routing/Menu, DocumentEditor/Notice6000ms and other37 Findings are unchanged. Prior broad allow entries describe historical delivery, not new write authority. Test fixture correction from target007 remains verification context; no new test/validator behavior is authorized by this amendment. No TASK011/012 or historical artifact edits.

## Evidence and acceptance

After exact approval and Gate checks, refreeze all existing delivery sources with new CSS hash and retain the current test verification manifest. Run required frontend verification and fresh independent source/verification review. At one normal native Windows Tauri window, retain150% DPI for the known failure, capture Light active, Dark active, Light active hover, keyboard focus active. Inspect the real rendered surface gap, ring continuity/clipping, unchanged2px active border, same Card/heading bounds before/after focus, no layout shift, normal title/background and no glyph/Badge/shadow/left stripe. Compare blurred/hover/focused captures; a continuous thicker blue band is FAIL regardless of correct CSS syntax. Static/computed values alone cannot establish visible separation.

If the bounded local inset does not yield the required gap, do not claim PASS; changes outside this focus-only treatment require STOP. No dark/native capture in this design-only turn. Stop after future four-image preview for USER 'SubscriptionCard active visual direction accepted'; only then run full matrix and later final independent/Human Visual/Task gates. This is not Golden Certification.

## Binding/provenance

Candidate005 refreshes its source manifest to current target007 sources; this is a readonly baseline refresh. No current implementation target is generated. DCR022 and native-review007 bind the single correction. Prior Candidates004/003/002/001, DCR020/021, canonical Contract, Task approvals, target005/006/007, source-review007 PASS and native-review007 REWORK are preserved. New exact Human approval is mandatory; this user's design-work authorization is not approval of the yet-unreviewed Candidate005 hash.
