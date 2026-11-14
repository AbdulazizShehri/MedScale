# Spec 036 limitations

- Synthetic fixtures are **not** a TheHalfMoon/MESC release and must never be treated as one.
- `Verified` ≠ product Pack admit; `product_admit_authorized` stays false while the gate is open.
- Publisher signature / trust roots are not implemented in Spec 036.
- Epoch store is in-process only (not durable across host restart).
- No brokered network fetch of real release assets.
- Spec 012 remains blocked on upstream released assets.
