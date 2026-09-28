# TASK-018 Candidate003 active-card design

Status: CANDIDATE; Human Change/Design Approval PENDING. DCR-021 defines activation sequence and identity rules. Only TASK018-HV-001/SF038 expected is revised; 37 other Findings unchanged.

## Implementation proposal
- Active Card: normal content-surface background, primary border2px; remove old selected background and check pseudo-element. Keep title inherited primary-text; no badge/shadow/left bar. Generic Row selected remains unchanged.
- Geometry: normal border1px + existing surface-padding14px gives15px inner inset; active border2px + calc(var(--surface-padding) - 1px) gives same15px. Existing border-box sizing/min-height/content width retained. The1px compensation is derived from border delta, not a new token or page layout value. Verify normal/active switching yields same outer and child bounds, including narrow and selection mode.
- Hover: inactive hover rule still excludes active; active normal content background/primary border remain under pointer hover.
- Focus: scoped SubscriptionCard :focus-visible rule independent of active state. Use2px primary outline inset by4px (derived two2px stroke widths) inside Card to remain visible despite existing grid overflow:hidden. Keep a visible surface gap separating focus outline from active border; no focus geometry jump. This local focus offset is part of the proposal, not a global focus change; do not change grid/DOM/handlers. On blur only focus outline disappears; active border remains. Selection-mode checkbox is existing batch-selection state, not an active glyph; do not remove or change it.
- Future source permission only src/styles.css for these active/hover/focus/border-padding declarations. No App.tsx/SidebarTraffic/DocumentEditor changes for this delta. Existing four-file delivery stays in full review/identity scope.

## Verification after published-candidate approval
Static cascade: active border/background/padding wins, no check pseudo-content, no active title color/shadow, hover exclusion, independent focus outline with clipping check. Native light/dark/keyboard/pointer/narrow/DPI: no layout shift, active remains distinct when focused/hovered; no batch selection semantics change. New implementation identity, fresh tests and FULL_SCOPE Review required. Current native evidence is historical only; no new screenshot or source implementation now.

## Contract / Task mapping
Candidate003 retains original full ui_scope/source_paths and Finding inventory provenance, with version0.3 as intended future binding. DCR021 overrides only old SF038 expectation; do not rewrite audit001/closure005. Separate Human Finding record supplies traceability rather than inserting a39th original audit Finding. This design is a visual Contract change, not business/architecture Material Change.
