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
# A Gatekeeper prompt can block `open` and AppleScript with nobody to answer it.
bounded() { perl -e 'alarm shift; exec @ARGV' "$@"; }

process_count() { pgrep -f "$app/Contents/MacOS/" | wc -l | tr -d ' '; }

window_query() {
  local pid
  pid="$(pgrep -f "$app/Contents/MacOS/" | head -1)"
  [ -n "$pid" ] || { echo "no process"; return 0; }
  bounded 20 osascript -e "tell application \"System Events\" to return count of windows of (first application process whose unix id is $pid)" 2>&1 | tr '\n' ' ' | cut -c1-200
}

window_count() {
  local answer
  answer="$(window_query | tr -d ' ')"
  case "$answer" in ''|*[!0-9]*) echo 0 ;; *) echo "$answer" ;; esac
}

wait_for_window() {
  local i
  for i in $(seq 1 90); do
    case "$(window_count)" in ""|0) sleep 1 ;; *) return 0 ;; esac
  done
  return 1
}

quit_app() {
  bounded 20 osascript -e 'tell application id "'"$identifier"'" to quit' >/dev/null 2>&1 || true
  local i
  for i in $(seq 1 60); do
    [ "$(process_count)" = "0" ] && return 0
    sleep 1
  done
  return 1
}

# Through LaunchServices as a person would. If `open` stalls, say so and run the
# app's executable, so a stall there doesn't hide whether the app itself starts.
# Stops only the app installed at this test's own path.
stop_test_app() { for one in $(pgrep -f "$app/Contents/MacOS/" || true); do kill "$one" 2>/dev/null || true; done; }

start_app() {
  local code=0 output
  output="$(bounded 45 open "$app" 2>&1)" || code=$?
  say "OPEN_EXIT=$code"
  [ -z "$output" ] || say "OPEN_OUTPUT=$(echo "$output" | tr '\n' ' ' | cut -c1-200)"
  if [ "$(process_count)" = "0" ]; then
    say "LAUNCH_METHOD=executable"
    "$app/Contents/MacOS/$(defaults read "$app/Contents/Info" CFBundleExecutable)" >/dev/null 2>&1 &
  else
    say "LAUNCH_METHOD=open"
  fi
}

[ ! -e "$app" ] || { say "APP_ALREADY_PRESENT"; exit 1; }
[ ! -e "$data" ] || { say "APP_DATA_ALREADY_PRESENT"; exit 1; }

say "ARCH=$(uname -m)"
say "DMG_SHA256=$(shasum -a 256 "$dmg" | cut -d' ' -f1)"

# As a browser marks a download.
xattr -w com.apple.quarantine "0081;$(printf %x "$(date +%s)");Safari;" "$dmg"
hdiutil verify "$dmg" >/dev/null && say "DMG_VERIFY=PASS"

# The DMG shows a licence prompt, which `yes` answers. A person sees it.
# `yes` ends on a closed pipe, which isn't a failure here; a missing mount is.
set +o pipefail
mount_point="$(yes | hdiutil attach -nobrowse -readonly "$dmg" | awk -F'\t' '/\/Volumes\//{print $NF; exit}')"
set -o pipefail
[ -n "$mount_point" ] || { say "DMG_ATTACH=FAIL"; exit 1; }
source_app="$(find "$mount_point" -maxdepth 1 -name '*.app' -print -quit)"
[ -n "$source_app" ] || { say "DMG_HAS_APP=FAIL"; exit 1; }
ditto "$source_app" "$app"
hdiutil detach "$mount_point" -quiet || hdiutil detach "$mount_point" -force
say "COPY_TO_APPLICATIONS=PASS"
mkdir -p "$(dirname "$report")/screens"
sips -s format png "$app/Contents/Resources/icon.icns" --out "$(dirname "$report")/screens/macos-app-icon.png" >/dev/null 2>&1 || say "ICON_EXPORT=unavailable"

say "QUARANTINE_ON_APP=$(xattr -p com.apple.quarantine "$app" >/dev/null 2>&1 && echo yes || echo no)"
codesign --verify --deep --strict "$app" >/dev/null 2>&1 && say "CODESIGN_VERIFY=PASS" || say "CODESIGN_VERIFY=FAIL"
say "CODESIGN_FLAGS=$(codesign -dv "$app" 2>&1 | sed -n 's/.*flags=\([^ ]*\).*/\1/p' | head -1)"
say "SPCTL=$(spctl --assess --type execute "$app" 2>&1 | tr '\n' ' ' | cut -c1-200)"

# Launch with quarantine cleared from this one app, which is what per-app approval
# does. This runs before the Gatekeeper test, which leaves a system prompt waiting.
saved_quarantine="$(xattr -p com.apple.quarantine "$app" 2>/dev/null || true)"
xattr -dr com.apple.quarantine "$app"
start_app
if wait_for_window; then
  say "LAUNCH_FROM_APPLICATIONS=PASS"
  mkdir -p "$(dirname "$report")/screens"
  screencapture -x "$(dirname "$report")/screens/macos-window-appeared.png" 2>/dev/null || say "SCREENSHOT=unavailable"
  sleep 8
  screencapture -x "$(dirname "$report")/screens/macos-first-launch.png" 2>/dev/null || say "SCREENSHOT=unavailable"
else
  say "LAUNCH_FROM_APPLICATIONS=FAIL"
  say "PROCESSES=$(process_count)"
  say "WINDOW_QUERY=$(window_query)"
  exit 1
fi
sleep 5
[ "$(process_count)" != "0" ] && say "ALIVE_AFTER_LAUNCH=PASS" || { say "ALIVE_AFTER_LAUNCH=FAIL"; exit 1; }

quit_app && say "QUIT=PASS" || { say "QUIT=FAIL"; exit 1; }
start_app
if wait_for_window; then say "REOPEN=PASS"; else say "REOPEN=FAIL"; exit 1; fi
quit_app || stop_test_app
sleep 3

# What Gatekeeper does with the build as downloaded. Recorded, not asserted.
xattr -w com.apple.quarantine "$saved_quarantine" "$app"
open_output="$(bounded 30 open "$app" 2>&1)" && open_code=0 || open_code=$?
sleep 15
say "QUARANTINED_OPEN_EXIT=$open_code"
say "QUARANTINED_OPEN_OUTPUT=$(echo "$open_output" | tr '\n' ' ' | cut -c1-200)"
say "QUARANTINED_OPEN_PROCESSES=$(process_count)"
say "QUARANTINED_OPEN_WINDOWS=$(window_count)"
stop_test_app
sleep 3

# Why a quarantined copy shows no window. A changed quarantine flag is a simulation,
# never user approval, and neither case can qualify a release. Each case stops only
# the processes it started, and only windows count as a launch.
pids_of_medscale() { pgrep -f 'MedScale\.app/Contents/MacOS/' | sort || true; }

quarantine_case() {
  local label="$1" flags="$2" dir copy before started=""
  dir="$(mktemp -d)"
  copy="$dir/MedScale.app"
  ditto "$app" "$copy"
  xattr -dr com.apple.quarantine "$copy"
  xattr -w com.apple.quarantine "$flags;$(printf %x "$(date +%s)");Safari;" "$copy"
  local spctl_code=0 spctl_output
  spctl_output="$(spctl --assess --type execute -vv "$copy" 2>&1)" || spctl_code=$?
  say "CASE_${label}_SPCTL_EXIT=$spctl_code"
  say "CASE_${label}_SPCTL=$(echo "$spctl_output" | tr '\n' ' ' | cut -c1-200)"
  say "CASE_${label}_SIGNATURE=$(codesign -dv "$copy" 2>&1 | grep -E '^(Signature|TeamIdentifier)' | tr '\n' ' ' | cut -c1-120)"
  local already pid="" windows=0 at_dyld="no" code=0 path="none"
  already="$(pids_of_medscale)"
  before="$(date '+%Y-%m-%d %H:%M:%S')"
  bounded 30 open "$copy" >/dev/null 2>&1 || code=$?
  sleep 20
  started="$(comm -13 <(echo "$already") <(pids_of_medscale) | tr '\n' ' ')"
  pid="${started%% *}"
  if [ -n "$pid" ]; then
    path="$(ps -o command= -p "$pid" | sed 's#^.*/T/##' | cut -c1-120)"
    windows="$(bounded 20 osascript -e "tell application \"System Events\" to return count of windows of (first application process whose unix id is $pid)" 2>/dev/null | tr -cd '0-9')"
    windows="${windows:-0}"
    bounded 20 sample "$pid" 1 2>/dev/null | grep -q '_dyld_start' && at_dyld="yes"
  fi
  local verdict
  if [ "$windows" -gt 0 ]; then verdict="WINDOW_VERIFIED"
  elif [ "$spctl_code" -ne 0 ]; then verdict="GATEKEEPER_BLOCKED"
  else verdict="APP_NO_WINDOW"; fi
  say "CASE_$label VERDICT=$verdict OPEN_EXIT=$code PID=${pid:-none} WINDOWS=$windows AT_DYLD_START=$at_dyld"
  say "CASE_${label}_PATH=$path"
  log show --start "$before" --style compact --predicate 'process == "syspolicyd" OR process == "amfid"' 2>/dev/null \
    | grep -iE 'MedScale|matching active rule|Prompt shown|AppleMobileFileIntegrityError' | tail -6 | sed 's#/private/var/folders/[^ ]*/T/#<tmp>/#g' | cut -c1-200 | sed "s/^/CASE_${label}_POLICY: /" | tee -a "$report" || true
  for one in $started; do kill "$one" 2>/dev/null || true; done
  sleep 3
  rm -rf "$dir"
}
quarantine_case DOWNLOADED_0081 0081
quarantine_case SIMULATED_01C1 01c1

rm -rf "$app"
[ ! -e "$app" ] && say "REMOVE_APP=PASS"
[ -e "$data" ] && say "USER_DATA_AFTER_REMOVAL=kept" || say "USER_DATA_AFTER_REMOVAL=none"
say "SIGNING=adhoc NOTARIZED=no"
say "RESULT=PASS"
