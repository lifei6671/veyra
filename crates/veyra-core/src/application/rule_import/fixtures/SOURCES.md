# Rule import fixtures

`youtube.mrs` and `private.mrs` are frozen original artifacts retrieved 2026-10-12 from the official MetaCubeX/meta-rules-dat `meta` branch. Tests replay them only over owned loopback HTTP; no external URL is contacted during tests.

- https://raw.githubusercontent.com/MetaCubeX/meta-rules-dat/meta/geo/geosite/youtube.mrs
- https://raw.githubusercontent.com/MetaCubeX/meta-rules-dat/meta/geo/geoip/private.mrs

`small.mrs` and `large.mrs` are local deterministic ASCII fixtures serialized according to the primary MRSv1/LOUDS implementation and compressed with local zstd only when preparing fixtures; production decoding uses locked ruzstd. No production dependency on the zstd executable.

`small.srs` and `large.srs` were produced by the existing fixed sing-box 1.14.0 `rule-set compile` CLI from owned input JSON. Production decompile only uses the same existing verified kernel and owned temporary workspace, never Runtime run/Controller.

Formal format sources:

- https://github.com/MetaCubeX/mihomo/blob/Meta/rules/provider/mrs_reader.go
- https://github.com/MetaCubeX/mihomo/blob/Meta/rules/provider/domain_strategy.go
- https://github.com/MetaCubeX/mihomo/blob/Meta/component/trie/domain_set_bin.go
- https://github.com/MetaCubeX/mihomo/blob/Meta/component/cidr/ipcidr_set_bin.go
- https://github.com/SagerNet/sing-box/blob/v1.14.0/cmd/sing-box/cmd_rule_set_decompile.go

Fixture SHA256:

- `large.mrs`: `288b14f1b19e37ff39f24593d1dfde78a97d4b88d6b7dcf2ee1f4fac35c0fffd`
- `large.srs`: `f04a59865e3bbd35d2d5fbc6391f10fa3d1ee58c5ec42ae1ecdaf52ac38797b0`
- `private.mrs`: `076ea1da10b442f01c6fb3b86a66411727a15df69d70e8d600a8a458b07a8bb0`
- `small.mrs`: `a195fab3a568ca4a7409792e7d2719d472a2e219cb70664dbe585e6dd48f952c`
- `small.srs`: `e1c669769095d12b56df96437c8032226f69fe9b387eaafc3198a99c85beb222`
- `youtube.mrs`: `e212b2ce9f39b30f4e2ab57da9ed0937673606bc56bb46e5d55ad7dd9af00aef`
