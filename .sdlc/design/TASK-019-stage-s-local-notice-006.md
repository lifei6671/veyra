# TASK-019 Stage S Dialog-local Notice design006

Status: CANDIDATE / exact Human Design and Adoption Approval pending. Sole finding: TASK019-STAGES-NATIVE-014-001. This is design and adoption preparation only; no source/test implementation, readiness, checkpoint or Native capture is authorized by this record.

## Basis and impact

Candidate005/DCR026revision007 permits only the prior Dialog width/border/fixture remediation. Native014 confirms that local Notice DOM containment does not prevent viewport positioning. This overlay adds only Dialog-local Notice presentation to that frozen write boundary; Contract0.4, R01–R10 and the complete S→A→B certification envelope remain identical. Historical allowances are retained for provenance and later stages, not reopened by this change.

Current source: SubscriptionPage.tsx:514 creates the existing alert/status noticeElement; :541–544 renders it as a direct child of section.subscription-dialog after renderDialog(). presentSubscriptionNotice (:56–58) and setNotice (:195–196) arbitrate local/global ownership. styles.css:389–430 applies fixed/left/bottom/z-index40 and viewport size limits; :684–704 supplies Dialog overflow:auto/max-height80dvh and 20px content insets. ToastHost child rules at :1191 onward are a separate selector/context. These source observations bind target013, not a new implementation.

Reference mapping: E:/wx_lifeilin/github.com/clash-verge-rev/src/components/profile/profile-viewer.tsx is the subscription form reference; src/components/base/base-dialog.tsx:58–72 composes DialogContent and DialogActions. Veyra keeps its approved local notice owner and existing DOM rather than adopting reference showNotice or MUI. Contract0.4 §4.1 provides --surface-padding=14px and --component-gap=10px; §6.6 permits Dialog padding18–20px; §4.2/4.3 and §8.2 provide existing popup/background/text/divider/error/radius tokens. No reference dependency or new token is introduced.

## Exact proposed CSS delta

Add one rule beside existing Subscription Dialog rules, after base notice declarations:

```css
.subscription-dialog > .subscription-notice {
  position: static;
  inset: auto;
  z-index: auto;
  display: block;
  width: auto;
  max-width: none;
  max-height: none;
  overflow: visible;
  overflow-wrap: anywhere;
  margin: 0 20px 20px;
  padding: var(--component-gap) var(--surface-padding);
}
```

The direct-child selector is deliberately narrower than descendant/shared Notice selectors. Normal block sizing fills the Dialog content width inside the existing 20px inset; border-box is already global. Natural height and wrapping remove viewport clamps and nested Notice scrolling. The Dialog retains its existing 80dvh overflow owner. Existing content padding separates the preceding form/footer from Notice, so no extra top gap is added. Existing border/error accent, popup radius, background, text and line-height remain inherited in both themes; the higher-specificity local max-width also removes the success-only viewport width cap locally. Global .subscription-notice and .toast-host selectors are unchanged.

The notice stays after existing content in reading and visual order. Long form/error content may require scrolling within Dialog; no auto-scroll, focus move, sticky placement, timer or lifecycle change is proposed. Existing role=alert/status remains. Native acceptance must confirm the message is reachable in the modal scroll area, wraps without horizontal overflow and does not overlay controls. CSS/source reasoning alone does not establish actual visible geometry or assistive-technology behavior.

No markup/class change is proposed. Only an independent Design Review demonstrating a concrete DOM/accessibility/layout failure of scoped CSS may request a new minimal markup/class candidate; such a finding does not authorize implementation. Keep noticeElement state, arbitration, validation/messages/submit handler, CRUD/persistence/activation/IPC/runtime, Toast duration/queue/owner/focus/Recovery Action, DocumentEditor, Proxies/Routing/Rust and dependencies unchanged.

## Focused regression design (not executed)

Protect only three behaviors: local validation remains in modal context while Dialog is open; Dialog-external Subscription feedback retains global Toast presentation; selectors do not contaminate each other. Extend only existing SubscriptionPage.test.ts infrastructure after separate implementation authorization. Retain actual presentSubscriptionNotice branch assertions; bind existing direct-child DOM composition and local-only CSS override, including static/inset/z-index/size reset and unchanged global/ToastHost positioning. Source/style assertions guard the CSS regression but cannot prove rendered geometry; do not label them Native evidence or fabricate a browser engine in Vitest.

Later authorized Native checks must reproduce the existing failing local-validation path, compare a legitimate global Subscription feedback path, and verify light/dark, wrapped text and modal scroll at the frozen Stage S environments. Keep the full existing certification matrix; these focused checks supplement rather than replace it. No new test infrastructure or test code in this design turn.

## Adoption and evidence boundary

Candidate006 is hash-bound to this overlay, DCR026revision008, authorization015, Impact014, NativeReview014, target013 and Candidate005/Approval012 provenance. Its Task snapshot is independently byte-hashed to avoid circular identity. Canonical TASK-019 remains Candidate005/BLOCKED until exact approval and byte-for-byte adoption. Human approval must name Candidate006 identity and snapshot hash and distinguish design/adoption from future implementation authority.

This turn runs design-phase machine checks and independent Design Review only. New design/adoption gate is pending Human; previous design/planning/readiness/checkpoint approvals remain valid historical observations only and cannot authorize this delta. After actual adoption, affected readiness and Stage S checkpoint must be evaluated before any separately authorized source change. Any source change needs a fresh source target, required frontend verification and complete current Native/Compliance/independent Delivery before Human Visual Approval. Stage A/B remain locked, no Golden PASS.

target013, NativeReview014 FAIL, the 34 screenshots and all existing Evidence are append-only. P1 stays OPEN until implementation and verification. toast-persistent applicability remains OPEN: current Subscription finite-duration consumers and primitive-only tests do not establish a legal native persistent path. Do not remove that frozen state, invent Subscription Apply recovery, waive coverage or relabel Native PASS. If no legal product path satisfies it, report a separate certification/Scope decision; this CSS change cannot resolve that issue implicitly.
