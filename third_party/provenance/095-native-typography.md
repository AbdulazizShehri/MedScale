# Spec 095 native typography admission

**Operation:** `VENDOR_SNAPSHOT`, immutable unmodified font assets only.

**Authority:** `specs/095-brand-foundation/spec.md`, explicit minimal font distribution policy.

**Acquired:** 2026-09-29, from official upstream at the exact revisions below.

| Local file | Exact upstream revision/path | SHA-256 |
|---|---|---|
| `assets/brand/fonts/InterVariable.ttf` | `rsms/inter@e3a3d4c57d5ecc01453a575621882a384c1995a3/docs/font-files/InterVariable.ttf` | `4989b125924991b90d05b2d16e0e388c48f7d5bb8b30539bbf9c755278d0ccaf` |
| `assets/brand/fonts/Inter-OFL.txt` | same revision, `LICENSE.txt` | `262481e844521b326f5ecd053e59b98c8b2da78c8ee1bdbb6e8174305e54935a` |
| `assets/brand/fonts/JetBrainsMonoNL-Regular.ttf` | `JetBrains/JetBrainsMono@cd5227bd1f61dff3bbd6c814ceaf7ffd95e947d9/fonts/ttf/JetBrainsMonoNL-Regular.ttf` | `fb3b2575d7b0657359707993288f12a7360344d39387bb26050e276d61f6bd2a` |
| `assets/brand/fonts/JetBrainsMono-OFL.txt` | same revision, `OFL.txt` | `30f0c136e3c88e422d0791acd97238870f9054a9729bc34cf2ff0d4ed8cac4ad` |

Official repositories: https://github.com/rsms/inter and https://github.com/JetBrains/JetBrainsMono . Inter tag v4.1 and JetBrains tag v2.304 were verified against GitHub's ref API. SHA-256 was measured on the saved bytes. Inter is 879708 bytes; JetBrains Mono NL Regular is 208576 bytes.

## Rights and distribution

Both upstream files carry SIL OFL 1.1 copyright/redistribution terms in their unchanged license files. This owning spec explicitly requires embedding because native UI rendering cannot depend on user-installed fonts or a network font service. Only the selected Roman variable UI file and regular NL metadata file are admitted; no italic, webfont or redundant static weights. Full copyright/license and FONT_NOTICE accompany the checksummed portable package. Project Apache licensing does not replace font licensing. Do not change reserved font names or distribute a modified font without a new admission.

## Placement, maintenance and exit

The admitted Slint parser/renderer consumes font data as presentation assets; fonts have no code, storage, key or network authority. Existing bounded native/font rendering tests and exact-head CI qualify the parser integration. Asset hash and portable license checks detect drift/omission. No transitive dependency or runtime download is added. Update only in a new spec with source/revision/hash/rights checks and native renders. Exit by replacing registered font imports and lockups in a qualified forward change; historical commits/assets remain provenance.
