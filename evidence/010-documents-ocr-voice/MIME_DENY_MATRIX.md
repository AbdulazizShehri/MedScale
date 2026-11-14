# Spec 010 MIME deny matrix

| MIME class | Default | Result |
|---|---|---|
| text/plain | admit | SourceRecord |
| application/pdf | admit | SourceRecord (no parse engine) |
| image/png, image/jpeg | admit | OCR stub eligible |
| audio/wav, audio/mpeg | admit | ASR stub eligible |
| application/octet-stream | quarantine | MimeDenied |
| text/html | quarantine | MimeDenied |
| office openxml | quarantine | MimeDenied until sandboxed worker |

No unsandboxed office/HTML parse in Spec 010.
