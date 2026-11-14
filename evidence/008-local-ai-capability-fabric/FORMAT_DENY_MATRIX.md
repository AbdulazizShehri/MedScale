# Format deny matrix (Spec 008)

| Kind | Default | Result |
|---|---|---|
| fixture_bytes | allow | PASS admit when digests match |
| tokenizer_meta | allow | allowed class (bounded metadata) |
| onnx_model | allow class | **not executed** — no ORT dep in 008 |
| pickle | deny | ForbiddenArtifactKind |
| code_bin | deny | ForbiddenArtifactKind |
| onnx_custom_op | deny | ForbiddenArtifactKind (unsigned) |
