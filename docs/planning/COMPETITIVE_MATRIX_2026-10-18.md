# Competitive matrix (2026-10-18)

**Sources:**
- OpenMed: the pinned v3.0.0 source, plus a head-to-head run on the same runner (#248).
- OpenEvidence (openevidence.com) and Abridge (abridge.com): public, unauthenticated marketing pages read on 2026-10-18. These are product-pattern references only; nothing was signed into, reverse engineered or copied (`SOURCE_REUSE_MATRIX_2026-10-18.md`).

**Not claimed:** a superiority claim needs measured evidence. The rows below state what each product publicly offers and what MedScale has evidence for.

| Capability | OpenMed v3.0.0 | OpenEvidence (public claims) | Abridge (public claims) | MedScale (evidence) | MedScale gap / next |
|---|---|---|---|---|---|
| Local, offline processing | Local after model download | Cloud service | Cloud service | Rust Core, vault-bound, no network at runtime; desktop on Windows/macOS | Linux desktop waits on upstream (`TAURI_LINUX_DESKTOP_GTK3`) |
| Clinical NER and PII | 2,266-row catalog, Python pipeline | Not a public product surface | Not a public product surface | 1,466 rows executed. Head-to-head pilot: higher strict F1 on 3/3 PII models and level on 2/2 NER models with word-level decoding; default decoding lower; warm latency 1.3–2.9× slower | Candle latency; choose the default decoding from evidence |
| Evidence-cited answers | Clinical brief demo with synthetic providers | Answers drawing on licensed NEJM, JAMA, NCCN, Nature and Cochrane content | Not the focus | Lexical retrieval over a synthetic corpus; MedAgent runs bound to chosen sources, with an evidence view | No licensed literature (rights gate); no generative answer path within the 1 GiB bound |
| Ambient documentation | Not a product surface | "Visits" product listed | Real-time capture, decision support, care gaps, notes, coding, orders, patient summaries | AudioFlow: immutable audio, VAD, governed transcripts and corrections; Spec 106 local Whisper ASR (qualification in progress) | Diarization, note drafting from a reviewed transcript, EHR write-back (gated) |
| Governance of AI output | Policy and guard modules | Not public | Public whitepapers on evaluation and hallucination | Every model output is a proposal; promotion is explicit; audit and disclosure records in Core | Clinician-in-the-loop evaluation needs authorized real data (not granted) |
| Multi-model comparison | Not a product surface | Not public | Not public | Model Fleet: independent lanes on chosen agents and Packs; comparison records agreement without choosing a winner | Fleet metrics over gold sets |
| Deployment and clients | Python, Swift, Android, browser | Web and mobile, free for verified US HCPs | Enterprise (300+ health systems claimed) | Desktop (unsigned engineering packages), CLI | Signing, release qualification and clinical validation stay gated |
