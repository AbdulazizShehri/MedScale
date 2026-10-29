"""Source-bound macOS WKWebView keyboard focus proof (disposable GitHub runner).

Copy-first adaptation of macos-webdriver-smoke.py. No patient data, files,
downloaded weights, or screenshot capture. Only real System Events Tab and
Shift+Tab key presses may satisfy the proof; JS focus() is prohibited.
"""
import json
import os
from pathlib import Path
import subprocess
import time
from urllib import request, error

if os.getenv("GITHUB_ACTIONS") != "true" or os.getenv("RUNNER_OS") != "macOS":
    raise SystemExit("Refusing keyboard qualification outside disposable macOS CI")

base = "http://127.0.0.1:4445"
paths = [Path("apps/desktop-tauri/src-tauri/target"), Path("target")]
bundles = [app for folder in paths if folder.exists()
           for app in folder.rglob("MedScale.app")
           if "/bundle/macos/" in str(app)]
if len(bundles) != 1:
    raise SystemExit(f"Expected exactly one disposable native app; found {len(bundles)}")
binary = bundles[0] / "Contents/MacOS/medscale-desktop-tauri"
if not binary.is_file():
    raise SystemExit("Ephemeral native app executable missing")


def http(method, route, data=None):
    payload = json.dumps(data).encode() if data is not None else None
    req = request.Request(base + route, data=payload, method=method)
    req.add_header("Content-Type", "application/json")
    try:
        with request.urlopen(req, timeout=20) as resp:
            body = json.loads(resp.read().decode())
    except error.HTTPError as exc:
        raise AssertionError(f"Native WebDriver HTTP {exc.code} {method} {route}") from exc
    return body.get("value", body)


def element(session, using, value, timeout=45):
    end = time.monotonic() + timeout
    while time.monotonic() < end:
        try:
            found = http("POST", f"/session/{session}/element",
                         {"using": using, "value": value})
            if isinstance(found, dict):
                key = "element-6066-11e4-a52e-4f735466cecf"
                if found.get(key) or found.get("ELEMENT"):
                    return found.get(key) or found["ELEMENT"]
        except Exception:
            pass
        time.sleep(1)
    raise AssertionError(f"Native UI element missing: {using} {value}")


def button(session, label):
    return element(session, "xpath",
                   f"//button[contains(normalize-space(.), '{label}')]")


def click(session, node):
    return http("POST", f"/session/{session}/element/{node}/click", {})


def evaluate(session, script):
    return http("POST", f"/session/{session}/execute/sync",
                {"script": script, "args": []})


def keypress(pid, code, shift=False):
    suffix = " using {shift down}" if shift else ""
    script = f'''tell application "System Events"
    set targetProcess to first application process whose unix id is {pid}
    set frontmost of targetProcess to true
    key code {code}{suffix}
end tell'''
    sent = subprocess.run(["osascript", "-e", script],
                          capture_output=True, text=True, timeout=15)
    if sent.returncode != 0:
        raise AssertionError("macOS System Events keyboard authorization unavailable: "
                             + sent.stderr.strip()[:220])


def focused(session):
    return evaluate(session, """const e=document.activeElement;return {
      id:e?.id||'',tag:e?.tagName||'',
      text:e?.textContent?.trim()||''
    };""")


def picker_windows_and_sheets(pid):
    """Copy-first AppKit window/sheet probe from macos-webdriver-smoke.py."""
    script = f'''tell application "System Events"
    set targetProcess to first application process whose unix id is {pid}
    set windowCount to count of windows of targetProcess
    set sheetCount to 0
    if windowCount > 0 then set sheetCount to count of sheets of window 1 of targetProcess
    return (windowCount as text) & "," & (sheetCount as text)
end tell'''
    result = subprocess.run(["osascript", "-e", script],
                            capture_output=True, text=True, timeout=12)
    if result.returncode != 0:
        raise AssertionError("macOS native picker window inspection unavailable: "
                             + result.stderr.strip()[:220])
    counts = result.stdout.strip().split(",")
    if len(counts) != 2 or not all(x.strip().isdigit() for x in counts):
        raise AssertionError("Unexpected real macOS window/sheet count")
    return tuple(int(x.strip()) for x in counts)


session = None
process = subprocess.Popen([str(binary)],
                           env=dict(os.environ, TAURI_WEBDRIVER_PORT="4445"),
                           stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
try:
    for _ in range(65):
        if process.poll() is not None:
            raise AssertionError("Native macOS app exited before WebDriver startup")
        try:
            if isinstance(http("GET", "/status"), dict):
                break
        except Exception:
            pass
        time.sleep(1)
    else:
        raise AssertionError("Native WebDriver localhost server unavailable")

    created = http("POST", "/session", {
        "capabilities": {"alwaysMatch": {"browserName": "chrome"}}
    })
    session = created.get("sessionId") if isinstance(created, dict) else None
    if not session:
        raise AssertionError("Native WebDriver session missing")
    click(session, button(session, "Try MedScale with sample data"))
    element(session, "xpath",
            "//button[contains(@class,'nav') and contains(normalize-space(.),'Audio')]")
    click(session, button(session, "Audio"))
    click(session, button(session, "Create a project"))
    field = element(session, "css selector", "#pn")
    http("POST", f"/session/{session}/element/{field}/value", {
        "text": "Keyboard qualification synthetic project",
        "value": list("Keyboard qualification synthetic project"),
    })
    click(session, button(session, "Create project"))
    click(session, button(session, "Audio"))
    button(session, "Import synthetic tone sample")
    print("MAC_NATIVE_KEYBOARD_SYNTHETIC_WORKSPACE_PASS")

    # Tauri must expose explicit labels as well as an actual native focus
    # target. No JS .focus() or synthetic KeyboardEvent is permitted.
    for field_id, expected_label, expected_button in [
        ("wp1", "Import a WAV recording", "Browse WAV"),
        ("ap1", "Admit an Audio Pack folder", "Browse folder"),
    ]:
        metadata = evaluate(session, f"""const e=document.getElementById('{field_id}');
          return {{isInput:e?.tagName==='INPUT',
            labels:[...(e?.labels||[])].map(l=>l.textContent.trim())}};""")
        if (not isinstance(metadata, dict) or not metadata.get("isInput")
                or expected_label not in metadata.get("labels", [])):
            raise AssertionError(f"Accessible HTML label is missing: {field_id}")
        # Runner-only non-existent sentinel: never import a customer file.
        suffix = "keyboard-audio.wav" if field_id == "wp1" else "keyboard-pack"
        expected_path = str(Path(os.environ["RUNNER_TEMP"]) / suffix)
        input_node = element(session, "css selector", f"#{field_id}")
        http("POST", f"/session/{session}/element/{input_node}/value", {
            "text": expected_path, "value": list(expected_path),
        })
        actual_path = evaluate(session,
                               f"return document.getElementById('{field_id}')?.value ?? null;")
        if actual_path != expected_path:
            raise AssertionError(f"Could not seed disposable input: {field_id}")
        click(session, input_node)
        if focused(session).get("id") != field_id:
            raise AssertionError(f"Mouse click failed to focus native input: {field_id}")

        keypress(process.pid, 48)  # Real macOS Tab hardware-style key event
        next_focus = focused(session)
        if (next_focus.get("tag") != "BUTTON" or
                expected_button not in next_focus.get("text", "")):
            raise AssertionError(
                f"Real macOS Tab did not focus {expected_button} after {field_id}: "
                f"tag={next_focus.get('tag')!r} text={next_focus.get('text')[:60]!r}")
        print("MAC_NATIVE_KEYBOARD_TAB_BUTTON_FOCUS_PASS", expected_button)

        keypress(process.pid, 48, shift=True)  # Real macOS Shift+Tab
        back_focus = focused(session)
        if back_focus.get("id") != field_id:
            raise AssertionError(f"Real macOS Shift+Tab did not restore input focus: {field_id}")
        print("MAC_NATIVE_KEYBOARD_SHIFT_TAB_RETURN_PASS", field_id)

        # Extend the proven focus route to an *actual keyboard-triggered*
        # native AppKit chooser. Adapt the picker counter + real Escape method
        # from macos-webdriver-smoke.py. No click(), JS event or fake dialog.
        keypress(process.pid, 48)
        target = focused(session)
        if (target.get("tag") != "BUTTON" or
                expected_button not in target.get("text", "")):
            raise AssertionError(f"Second real Tab lost Browse focus: {field_id}")
        baseline = picker_windows_and_sheets(process.pid)
        keypress(process.pid, 49)  # Real macOS Space on the focused Browse button
        opened = False
        try:
            for _ in range(20):
                counts = picker_windows_and_sheets(process.pid)
                if counts[0] > baseline[0] or counts[1] > baseline[1]:
                    opened = True
                    break
                time.sleep(1)
        finally:
            # Do not leave native sheet open even on an assertion failure.
            keypress(process.pid, 53)  # Real PID-targeted macOS Escape
        if not opened:
            raise AssertionError(f"Real Space did not open native chooser: {expected_button}")
        dismissed = False
        escape_count = 1
        for attempt in range(2):
            for _ in range(10 if attempt == 0 else 20):
                counts = picker_windows_and_sheets(process.pid)
                if counts[0] <= baseline[0] and counts[1] <= baseline[1]:
                    dismissed = True
                    break
                time.sleep(1)
            if dismissed:
                break
            if attempt == 0:
                keypress(process.pid, 53)
                escape_count = 2
        if not dismissed:
            raise AssertionError(
                f"Native chooser remained after {escape_count} real Escape keys: {expected_button}"
            )
        print("MAC_NATIVE_KEYBOARD_SPACE_OPENS_OS_PICKER_PASS", expected_button)
        print("MAC_NATIVE_KEYBOARD_ESCAPE_DISMISSES_OS_PICKER_PASS", expected_button)
        retained = False
        for _ in range(20):
            actual_path = evaluate(session,
                                   f"return document.getElementById('{field_id}')?.value ?? null;")
            browse_node = button(session, expected_button)
            if (actual_path == expected_path and
                    http("GET", f"/session/{session}/element/{browse_node}/enabled") is True):
                retained = True
                break
            time.sleep(1)
        if not retained:
            raise AssertionError(f"Keyboard picker cancellation lost input or control: {field_id}")
        print("MAC_NATIVE_KEYBOARD_PICKER_CANCEL_RETAINS_PATH_PASS", field_id)

    print("MAC_NATIVE_KEYBOARD_LABELS_AND_FOCUS_ORDER_PASS")
finally:
    if session:
        try:
            http("DELETE", f"/session/{session}")
        except Exception:
            pass
    process.terminate()
    try:
        process.wait(timeout=12)
    except subprocess.TimeoutExpired:
        process.kill()
