# MedScale public landing page

This is the static, dependency-free, English-language MedScale open-source landing page. Its purpose is to explain the engineering preview and link to the public source and contribution paths. It is **not** a clinical deployment, desktop release, regulatory certification, or real-patient-data authorization.

- Preview locally: \`python3 -m http.server 8765 --directory site\`.
- Validate: \`python3 site/check_site.py\`.
- Brand assets are copied unmodified from the repository's approved \`assets/brand/\` masters. Do not recolor or reconstruct the logo.
- Only publish claims supported by \`PRODUCT.md\` and \`docs/qualification/EXTERNAL_GATES.md\`. The UI illustration is explicitly labeled conceptual and synthetic.
- No client-side tracking, external fonts, analytics, JavaScript, network calls, paid services, or patient data.
- GitHub Actions validates pull requests. Only a push to protected \`main\` can deploy to GitHub Pages, after Pages is configured with **Source: GitHub Actions** in repository Settings → Pages.
- The site uses relative asset links and can be hosted at the standard project Pages URL or under a custom path. Custom domain configuration is separate.

Publishing this site does not change \`RELEASE_READY=false\` or \`REAL_PHI_AUTHORIZATION=NOT_AUTHORIZED\`.
