# Allowlist deny matrix (Spec 013)

| Case | Expected reason | Transport sent |
|---|---|---|
| Empty allowlist | EmptyAllowlist | no |
| Unknown host | UnknownDestination | no |
| Purpose mismatch | PurposeMismatch | no |
| RealPhiForbidden | DataClassRefused | no |
| Disabled entry only | UnknownDestination | no |
| Match host+path+purpose+class | Allow / TransportFixtureOk | yes (fixture) |
| Live partner adapter | ExternalGateRequired | no |
