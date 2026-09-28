---
id: DCR-022
status: PROPOSED
change_source: USER confirmed TASK018-HV001-NATIVE-001
affected_task: [TASK-018]
---
# DCR-022: TASK-018 SubscriptionCard focus visual separation correction

## Frozen assumption and evidence

Candidate004 sha256:f76e9e5d172737ae95a590b48c36727792821de591510de185d7059ecea7d05c binds TASK-018-active-card-design-003.md: active border2px primary, focus2px primary outline at outline-offset:-4px, assumed to leave a visible content-surface gap. Native independent Review .sdlc/evidence/TASK-018/native-preview-review-007.json (sha256:18545c4e13a4ea7444b7faa63aa2826a864bab071213cec3958f6e4ed922b55e) on target sha256:fbfe465d28c9453239307fc38f974cf44f4e3f4cd2020c8171659383ef7f786c records P1 TASK018-HV001-NATIVE-001: real Windows Tauri1280x720 at150% DPI merges outline and border into a continuous thicker blue band. The layout is stable but focus is not independently visible.

## Proposed correction

Preserve canonical UI Contract0.3 sha256:bd598a07008e6fc4bf210a412289d6c60d9e5f5f5dd372e4cd47090b301c86c0, DCR021 active language, active2pxprimary border, normal content surface, no check/Badge/shadow/leftstripe/primarytitle. Revise only the frozen local focus implementation assumption: require a visible content-surface gap between active outer border and independently visible focus ring in real Tauri; focus must not merely thicken active border. Freeze no-layout-shift and independent visibility outcomes, not an exact offset. A2px primary outline with -6px offset is the first bounded implementation trial, NOT an already validated final value and NOT a global token. The exact local inset may be adjusted only to satisfy these outcomes within the same single-file focus treatment after approval; if this requires other properties beyond the allowed focus treatment, stop for scope review.

## Impact and authority

Only src/styles.css SubscriptionCard :focus-visible / directly related focus separation may change after exact DCR and Candidate approval. No active border/background/padding semantic change, hover change, DOM/TSX/handler/state, Sidebar/Proxies/Routing/Menu/Notice6000ms, other tokens or37 Findings. No wrapper, framework, shadow or new colors. Current turn design/review only.

This DCR is necessary because an approved implementation design assumption must change; no new product Requirement or business/architecture Material Change. No Contract0.4, no change to DCR021/Contract0.3, no new Finding in original38 inventory. Task and old approvals/targets/reviews/screenshots remain historical identities. Only the affected focus assumption in design003 and its adoption by execution004 is superseded after approval; all other prior decisions remain.

## Gates, compatibility and rollback

Fresh Candidate005 design validator + independent Technical Design Review -> exact Human Change/Design Approval -> legal Task binding/readiness/checkpoint -> bounded CSS correction -> new full delivery/source and verification identity -> fresh verification/review -> four-image real Tauri preview including the failing150% focus environment -> USER direction confirmation -> full DPI/window/state matrix and final Human Gates. Existing source Review PASS remains historical; native preview REWORK is not rewritten. An unsuccessful inset trial is not compliance PASS; retain failures and do not move Gates. No data migration/runtime/release effect; prior source is retained in target007 provenance and cannot be reused as visual PASS. Full delivery requirements from Candidate004 stay in force.

See .sdlc/design/TASK-018-focus-separation-amendment-005.md for exact acceptance and scope. TASK012 and Golden chain remain blocked; no automatic recertification or Task completion.
