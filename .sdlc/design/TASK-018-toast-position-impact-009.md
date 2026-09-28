# TASK-018 Toast positioning — bounded impact analysis

Source: TASK018-HV-004; current USER request. Planning only; no production change.

## Current authorities and conflict

Canonical Contract 0.3 remains sha256:bd598a07008e6fc4bf210a412289d6c60d9e5f5f5dd372e4cd47090b301c86c0. Its §5 freezes Toast width/behavior and §10 says not to obscure primary operations; it does not freeze a bottom-left anchor. Top-right does not require changing that Contract. Candidate008 explicitly denies styles.css writes and other source writes, so adopting this independent concern requires a tracked frozen-scope correction, preserving Candidate008/target014 and all review identities.

## Actual implementation

- src/styles.css:388–430: failure-toast bottom-right, subscription-notice later overrides to bottom-left; existing --notice-edge = 20px and --notice-gap = 10px.
- src/styles.css:1136: proxy-routing-notice independently fixed bottom-right.
- src/App.tsx:230 owns its failure Toast; SubscriptionPage.tsx:91/500/529/536 owns one Notice rendered in page or dialog; DocumentEditor.tsx:113 renders its own error; ProxiesPage.tsx:75 and RoutingPage.tsx:30 render their Notice.
- SubscriptionPage owns one nullable Notice, not a history/queue. Replacing it removes the prior message; CSS cannot stack messages no longer rendered.
- Existing real test window uses the native decorated titlebar, outside the WebView client viewport. A top:20px client-viewport anchor would avoid those native controls. Native screenshots must still verify all requested DPI values.

## Minimal paths and unresolved scope

1. Position-only path: change only the three existing Toast selector families in src/styles.css to top/right existing notice-edge; clear bottom/left. Preserve colors, radius, widths, timers, handlers and all SubscriptionCard CSS. This meets positioning but cannot honestly meet continuous multi-message stacking.
2. Complete requested behavior requires a small shared presentation host with vertical layout and integration of existing render owners. Preserving successive messages additionally needs display-queue lifetime rules: replacing the current single slot with a queue is beyond position/style configuration. No library/framework needed, but new component and owner call-site authority must be explicitly bounded before Candidate creation. Dialog/top-layer accessibility and recovery action focus also need design treatment; do not silently portal modal alerts outside the dialog.

Recommended action: obtain the bounded scope decision for shared display host/queue and existing owner adapters, then create minimal DCR and a new Candidate for independent Design Review. Do not falsely mark stacking PASS from one visible message. Do not change protected production files under Candidate008 meanwhile.

## Preserved boundaries

No change to runtimeActive, pending arbitration, activationCompletion, PendingActivation metadata, useLayoutEffect completion, invoke sequencing, Backend/IPC/Runtime/Domain/persistence/observation transport, Card presentation, Notice6000ms duration, business messages or error recovery. No TASK012/Golden work, no full failure matrix expansion. No production source edited by this analysis.

## Verification and identity consequences

After approved implementation, freeze a new target, run relevant existing frontend checks and independent source review. Re-capture success and StateUnavailable Toast at top-right; concurrent/successive visible messages stack downward without overlap or page shift; preserve primary-card access and native controls; light/dark at 100/125/150 percent DPI. Recheck old-active → target-pending → failure → old-active-restored and zero dual emphasis. This positioning matrix does not authorize stop/start/recovery fault campaigns. Historical target014 failure PASS remains true for its old positioning; new Toast evidence and affected Delivery evidence require fresh identity. Final Human Visual approval remains separate.
