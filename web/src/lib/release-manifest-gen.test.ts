import test from "node:test";
import assert from "node:assert/strict";
import { checkedManifest, manifestFromInventory, type Inventory } from "./release-manifest-gen.ts";

/** The 19 files of release candidate 0.1.0-beta.1 (docs/release/RELEASE_CANDIDATE_0.1.0-beta.1.md). */
const RC: Inventory = {
  schema: 1,
  version: "0.1.0-beta.1",
  source_sha: "6d0279e93fd011b719652a14f6cfa74dc82fad1c",
  run: "https://github.com/AbdulazizShehri/MedScale/actions/runs/43247164670",
  artifacts: [
    { name: "MedScale_0.1.0-beta.1_SBOM.cdx.json", platform: "all", arch: "all", kind: "sbom", size: 330821, sha256: "e6d5b45f4c9d50318d4d3213a82e108d828a73ae634e8fade666718f75a5906c", signing: "n/a" },
    { name: "MedScale_0.1.0-beta.1_linux-arm64-notices.zip", platform: "linux", arch: "arm64", kind: "notices", size: 3560195, sha256: "c80bb183e52bc8f2b6830da7fe8737f7e85d4f42341b223617c20a7f23655303", signing: "n/a" },
    { name: "MedScale_0.1.0-beta.1_linux-x64-notices.zip", platform: "linux", arch: "x64", kind: "notices", size: 3560192, sha256: "173195f3cdfd8c52a8823d781891827bbe79496651810fc9abe222e3c981bb67", signing: "n/a" },
    { name: "MedScale_0.1.0-beta.1_macos-arm64-notices.zip", platform: "macos", arch: "arm64", kind: "notices", size: 3560456, sha256: "cdedc1d5f38e55f74d9ac171d2b32dd9e3d207747b45044f21e9244b41a2304f", signing: "n/a" },
    { name: "MedScale_0.1.0-beta.1_macos-x64-notices.zip", platform: "macos", arch: "x64", kind: "notices", size: 3560465, sha256: "62f47a2cba2f885cf2db72a7de8bf4a0997a281e13b8aa2a56534764ef8d7664", signing: "n/a" },
    { name: "MedScale_0.1.0-beta.1_windows-x64-notices.zip", platform: "windows", arch: "x64", kind: "notices", size: 3560767, sha256: "de175a0c78fe33dbcd9dd501c06fb57bfdfd84dec27db82bdce2f40b09ac4839", signing: "n/a" },
    { name: "MedScale_0.1.0_aarch64.AppImage", platform: "linux", arch: "arm64", kind: "appimage", size: 98298376, sha256: "35b3efcdffa2e91e02afd18f4bec5ede521af35318be63314f6a8614b1a3e12a", signing: "unsigned" },
    { name: "MedScale_0.1.0_aarch64.dmg", platform: "macos", arch: "arm64", kind: "dmg", size: 21140979, sha256: "e7774cd442a9c1586430284d8eb3506622a48825fe7ef8fbec872c0cfa4c11b0", signing: "ad-hoc (not Developer ID, not notarized)" },
    { name: "MedScale_0.1.0_amd64.AppImage", platform: "linux", arch: "x64", kind: "appimage", size: 99944952, sha256: "4223ea2e2c2a7a59e9fdcd8f9b22159580267ae0660a1c0733502a6abe39dde3", signing: "unsigned" },
    { name: "MedScale_0.1.0_amd64.deb", platform: "linux", arch: "x64", kind: "deb", size: 24107016, sha256: "f8e1936e819e2b6561a96a5231a13ba350c5698c03ab6061e1e06f592fdf85bc", signing: "unsigned" },
    { name: "MedScale_0.1.0_arm64.deb", platform: "linux", arch: "arm64", kind: "deb", size: 24506814, sha256: "54e6ef0a2fc89f77c9540b665c3a1d83b1d5b660472f8a16320ec036add08e09", signing: "unsigned" },
    { name: "MedScale_0.1.0_x64-portable.zip", platform: "windows", arch: "x64", kind: "portable", size: 24677882, sha256: "492f5a604ec55ae5e26d05c2471ab7b628c4218025f1bd74048400a20d86aa16", signing: "unsigned" },
    { name: "MedScale_0.1.0_x64-setup.exe", platform: "windows", arch: "x64", kind: "installer", size: 14280170, sha256: "e922e47360d87681cd80a8c1f7b3f9e07e2c95d8b9eef82a254731d8fbdfbe22", signing: "unsigned" },
    { name: "MedScale_0.1.0_x64.dmg", platform: "macos", arch: "x64", kind: "dmg", size: 23161269, sha256: "c8bf3d25ca248edf84b529bb2de8dd4c195ea22843fa3ee54575325322a70279", signing: "ad-hoc (not Developer ID, not notarized)" },
    { name: "medscale-0.1.0-aarch64-apple-darwin.tar.gz", platform: "cli", arch: "aarch64-apple-darwin", kind: "cli", size: 21874028, sha256: "82fd44696b0fe09e406e7c483ed0e005e46d20a5e197c63a8cbc83093400aae3", signing: "unsigned" },
    { name: "medscale-0.1.0-aarch64-unknown-linux-gnu.tar.gz", platform: "cli", arch: "aarch64-unknown-linux-gnu", kind: "cli", size: 24536929, sha256: "edc53fc45167536ac16e81dea3dc8730abde2e2fcbef9db7faecd89ee7fc9319", signing: "unsigned" },
    { name: "medscale-0.1.0-x86_64-apple-darwin.tar.gz", platform: "cli", arch: "x86_64-apple-darwin", kind: "cli", size: 23047287, sha256: "3e2da9809915d1270b2dbc35e491206e5fbc4131d18f3b688888f1b876a7f09a", signing: "unsigned" },
    { name: "medscale-0.1.0-x86_64-pc-windows-msvc.zip", platform: "cli", arch: "x86_64-pc-windows-msvc", kind: "cli", size: 24393665, sha256: "630f4e021e4766c9910628e6107adb3888d4fda6bb1c8e1d19d9b2b80607da30", signing: "unsigned" },
    { name: "medscale-0.1.0-x86_64-unknown-linux-gnu.tar.gz", platform: "cli", arch: "x86_64-unknown-linux-gnu", kind: "cli", size: 24217975, sha256: "6035f4a8124d1c82a9c923cb760bfa528e84dad992f1416ec1c2542083eb7f7b", signing: "unsigned" },
    ...(["windows-x64", "macos-arm64", "macos-x64", "linux-x64", "linux-arm64"] as const).flatMap((pa) => {
      const [platform, arch] = pa.split("-");
      return [
        { name: `MedScale_0.1.0-beta.1_${pa}-package-manifest.json`, platform, arch, kind: "package-manifest", size: 9000, sha256: "1".repeat(64), signing: "n/a" },
        { name: `MedScale_0.1.0-beta.1_${pa}-SBOM.cdx.json`, platform, arch, kind: "platform-sbom", size: 300000, sha256: "2".repeat(64), signing: "n/a" },
        { name: `MedScale_0.1.0-beta.1_${pa}-NOTICE.md`, platform, arch, kind: "platform-notice", size: 4000, sha256: "3".repeat(64), signing: "n/a" },
      ];
    }),
  ],
};
const APPROVAL = "https://github.com/AbdulazizShehri/MedScale/issues/436#issuecomment-123456789";

test("the candidate's 13 downloads become a valid manifest", () => {
  const { release } = checkedManifest(RC, { tag: "v0.1.0-beta.1", founderApproval: APPROVAL });
  assert.equal(Object.keys(release.platforms).length, 13);
  const win = release.platforms.windows!;
  assert.equal(win.fileName, "MedScale_0.1.0_x64-setup.exe");
  assert.equal(win.sha256, "e922e47360d87681cd80a8c1f7b3f9e07e2c95d8b9eef82a254731d8fbdfbe22");
  assert.equal(win.signing, "unsigned");
  assert.equal(win.url, "https://github.com/AbdulazizShehri/MedScale/releases/download/v0.1.0-beta.1/MedScale_0.1.0_x64-setup.exe");
  assert.equal(win.companions.checksums, "SHA256SUMS");
  assert.equal(win.companions.notices, "MedScale_0.1.0-beta.1_windows-x64-NOTICE.md");
  assert.equal(win.companions.packageManifest, "MedScale_0.1.0-beta.1_windows-x64-package-manifest.json");
  assert.equal(win.companions.sbom, "MedScale_0.1.0-beta.1_windows-x64-SBOM.cdx.json");
  // A portable zip is checked against the checksums file, the SBOM and the notices archive.
  const portable = release.platforms["windows-portable"]!;
  assert.equal(portable.companions.notices, "MedScale_0.1.0-beta.1_windows-x64-notices.zip");
  assert.equal(portable.companions.packageManifest, undefined);
  assert.equal(release.platforms.macos!.signing, "ad-hoc");
  assert.equal(release.platforms["cli-macos-x64"]!.companions.notices, "MedScale_0.1.0-beta.1_macos-x64-notices.zip");
  assert.equal(release.sourceSha, RC.source_sha);
});

test("a tag that isn't the candidate's version is refused", () => {
  assert.throws(() => manifestFromInventory(RC, { tag: "v0.1.0-beta.2", founderApproval: APPROVAL }), /doesn't match/);
});

test("an unknown file or a missing notices archive is refused", () => {
  const extra = { ...RC, artifacts: [...RC.artifacts, { name: "MedScale_0.1.0_x64.msi", platform: "windows", arch: "x64", kind: "installer", size: 2_000_000, sha256: "b".repeat(64), signing: "unsigned" }] };
  assert.throws(() => manifestFromInventory(extra, { tag: "v0.1.0-beta.1", founderApproval: APPROVAL }), /matches 0 download rules/);
  const noNotices = { ...RC, artifacts: RC.artifacts.filter((a) => a.name !== "MedScale_0.1.0-beta.1_linux-arm64-notices.zip") };
  assert.throws(() => manifestFromInventory(noNotices, { tag: "v0.1.0-beta.1", founderApproval: APPROVAL }), /notices archive for linux arm64/);
});

test("an approval that isn't a comment in this repository fails validation", () => {
  assert.throws(() => checkedManifest(RC, { tag: "v0.1.0-beta.1", founderApproval: "https://example.com/ok" }), /invalid/);
});
