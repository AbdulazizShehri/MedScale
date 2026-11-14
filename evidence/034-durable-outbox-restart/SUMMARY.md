# Summary — Spec 034 Durable Outbox Restart

1. SyntheticVault restart fixture reloads ExternalActionIntent via Spec 016 audit persistence.
2. UNKNOWN still requires reconcile after reload.
3. Doctor: `outbox_restart_qualified=true`; `nphies_authorized=false`.
4. No live partner transport; EncryptedVault authority sync out of scope.
