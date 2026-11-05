#!/usr/bin/env bash
# Installs MedScale from the actual DMG on a disposable macOS runner, as a person
# would after a browser download. Gatekeeper's verdict on a quarantined build is
# recorded, not asserted. It never disables Gatekeeper for the machine: quarantine
# is cleared from this one app only, which is what the per-app approval does.
set -euo pipefail
[ "${GITHUB_ACTIONS:-}" = "true" ] && [ "${RUNNER_ENVIRONMENT:-}" = "github-hosted" ] || { echo "disposable GitHub-hosted runner required"; exit 2; }

dmg="$1"
report="$2"
app="/Applications/MedScale.app"
identifier="org.medscale.desktop.preview"
data="$HOME/Library/Application Support/$identifier"
mkdir -p "$(dirname "$report")"
say() { echo "$*" | tee -a "$report"; }

process_count() { pgrep -f "$app/Contents/MacOS/" | wc -l | tr -d ' '; }

window_count() {
  osascript -e 'tell application "System Events" to count (windows of every process whose bundle identifier is "'"$identifier"'")' 2>/dev/null \
    | tr -d ', ' || echo 0
}

wait_for_window() {
  local i
  for i in $(seq 1 90); do
    case "$(window_count)" in ""|0) sleep 1 ;; *) return 0 ;; esac
  done
  return 1
}

quit_app() {
  osascript -e 'tell application id "'"$identifier"'" to quit' >/dev/null 2>&1 || true
  local i
  for i in $(seq 1 60); do
    [ "$(process_count)" = "0" ] && return 0
    sleep 1
  done
  return 1
}

[ ! -e "$app" ] || { say "APP_ALREADY_PRESENT"; exit 1; }
[ ! -e "$data" ] || { say "APP_DATA_ALREADY_PRESENT"; exit 1; }

say "ARCH=$(uname -m)"
say "DMG_SHA256=$(shasum -a 256 "$dmg" | cut -d' ' -f1)"

# As a browser marks a download.
xattr -w com.apple.quarantine "0081;$(printf %x "$(date +%s)");Safari;" "$dmg"
hdiutil verify "$dmg" >/dev/null && say "DMG_VERIFY=PASS"

# The DMG shows a licence prompt, which `yes` answers. A person sees it.
mount_point="$(yes | hdiutil attach -nobrowse -readonly "$dmg" | awk -F'\t' '/\/Volumes\//{print $NF; exit}')"
[ -n "$mount_point" ] || { say "DMG_ATTACH=FAIL"; exit 1; }
source_app="$(find "$mount_point" -maxdepth 1 -name '*.app' -print -quit)"
[ -n "$source_app" ] || { say "DMG_HAS_APP=FAIL"; exit 1; }
ditto "$source_app" "$app"
hdiutil detach "$mount_point" -quiet || hdiutil detach "$mount_point" -force
say "COPY_TO_APPLICATIONS=PASS"

say "QUARANTINE_ON_APP=$(xattr -p com.apple.quarantine "$app" >/dev/null 2>&1 && echo yes || echo no)"
codesign --verify --deep --strict "$app" >/dev/null 2>&1 && say "CODESIGN_VERIFY=PASS" || say "CODESIGN_VERIFY=FAIL"
say "CODESIGN_FLAGS=$(codesign -dv "$app" 2>&1 | sed -n 's/.*flags=\([^ ]*\).*/\1/p' | head -1)"
say "SPCTL=$(spctl --assess --type execute "$app" 2>&1 | tr '\n' ' ' | cut -c1-200)"

# What Gatekeeper does with the build as downloaded. Recorded either way.
open "$app" >/dev/null 2>&1 && open_code=0 || open_code=$?
sleep 15
say "QUARANTINED_OPEN_EXIT=$open_code"
say "QUARANTINED_OPEN_PROCESSES=$(process_count)"
say "QUARANTINED_OPEN_WINDOWS=$(window_count)"
quit_app || true
pkill -f "CoreServicesUIAgent" >/dev/null 2>&1 || true
pkill -f "$app/Contents/MacOS/" >/dev/null 2>&1 || true
sleep 3

# The per-app approval: clears quarantine from this app only.
xattr -dr com.apple.quarantine "$app"
open "$app"
if wait_for_window; then say "LAUNCH_FROM_APPLICATIONS=PASS"; else say "LAUNCH_FROM_APPLICATIONS=FAIL"; exit 1; fi
sleep 5
[ "$(process_count)" != "0" ] && say "ALIVE_AFTER_LAUNCH=PASS" || { say "ALIVE_AFTER_LAUNCH=FAIL"; exit 1; }

quit_app && say "QUIT=PASS" || { say "QUIT=FAIL"; exit 1; }
open "$app"
if wait_for_window; then say "REOPEN=PASS"; else say "REOPEN=FAIL"; exit 1; fi
quit_app || pkill -f "$app/Contents/MacOS/" || true

rm -rf "$app"
[ ! -e "$app" ] && say "REMOVE_APP=PASS"
[ -e "$data" ] && say "USER_DATA_AFTER_REMOVAL=kept" || say "USER_DATA_AFTER_REMOVAL=none"
say "SIGNING=adhoc NOTARIZED=no"
say "RESULT=PASS"
