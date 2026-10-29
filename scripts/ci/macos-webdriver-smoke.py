"""Native macOS WKWebView W3C browser smoke on a disposable GitHub runner.

No screenshots, personal data, or use of local founder devices; optional pinned public model runs only on disposable CI.
Requires feature-gated plugin instrumented only in runner workspace.
"""
import json
import os
from pathlib import Path
import subprocess
import time
from urllib import request, error

assert os.getenv("GITHUB_ACTIONS") == "true"
assert os.getenv("RUNNER_OS") == "macOS"
base = "http://127.0.0.1:4445"
paths = [Path("apps/desktop-tauri/src-tauri/target"), Path("target")]
bundles = [a for folder in paths if folder.exists()
           for a in folder.rglob("MedScale.app")
           if "/bundle/macos/" in str(a)]
if len(bundles) != 1:
    raise SystemExit(f"Expected one disposable MedScale.app build; found {len(bundles)}")
binary = bundles[0] / "Contents/MacOS/medscale-desktop-tauri"
if not binary.is_file():
    raise SystemExit("Native Tauri executable missing")

def http(method, path, data=None):
    payload = json.dumps(data).encode() if data is not None else None
    req = request.Request(base + path, data=payload, method=method)
    req.add_header("Content-Type", "application/json")
    try:
        with request.urlopen(req, timeout=20) as resp:
            body = json.loads(resp.read().decode())
    except error.HTTPError as exc:
        diagnostic = exc.read().decode(errors="replace")[:270]
        raise RuntimeError(f"WebDriver HTTP {exc.code} on {method} {path}: {diagnostic}") from exc
    return body.get("value", body)

def await_element(sid, using, value, timeout=50):
    end = time.monotonic() + timeout
    last = ""
    while time.monotonic() < end:
        try:
            e = http("POST", f"/session/{sid}/element", {"using": using, "value": value})
            if isinstance(e, dict):
                key = "element-6066-11e4-a52e-4f735466cecf"
                if e.get(key) or e.get("ELEMENT"):
                    return e.get(key) or e["ELEMENT"]
        except Exception as ex:
            last = str(ex)[:120]
        time.sleep(1)
    raise AssertionError(f"Native WKWebView element absent: {using} {value}: {last}")

def button(sid, label):
    xpath = f"//button[contains(normalize-space(.), '{label}')]"
    return await_element(sid, "xpath", xpath)

def click(sid, element):
    return http("POST", f"/session/{sid}/element/{element}/click", {})

session = None
env = dict(os.environ, TAURI_WEBDRIVER_PORT="4445")
process = subprocess.Popen([str(binary)], env=env,
                           stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
try:
    for i in range(65):
        if process.poll() is not None:
            raise SystemExit(f"Test-only native MedScale exited before WebDriver: {process.returncode}")
        try:
            status = http("GET", "/status")
            if isinstance(status, dict):
                print("MAC_NATIVE_WEBDRIVER_SERVER_STARTED")
                break
        except Exception:
            pass
        time.sleep(1)
    else:
        raise SystemExit("Embedded WebDriver server unavailable on localhost:4445")
    created = http("POST", "/session", {
        "capabilities": {"alwaysMatch": {"browserName": "chrome"}}
    })
    session = created.get("sessionId") if isinstance(created, dict) else None
    if not session:
        raise SystemExit(f"Embedded server did not return W3C session (keys={list(created) if isinstance(created,dict) else 'invalid'})")
    print("MAC_NATIVE_WEBDRIVER_SESSION_PASS")

    click(session, button(session, "Try MedScale with sample data"))
    await_element(session, "xpath", "//button[contains(@class, 'nav') and contains(normalize-space(.), 'Audio')]")
    print("MAC_NATIVE_WEBDRIVER_SAMPLE_WORKSPACE_PASS")

    click(session, button(session, "Audio"))
    click(session, button(session, "Create a project"))
    field = await_element(session, "css selector", "#pn")
    http("POST", f"/session/{session}/element/{field}/value",
         {"text": "MedScale CI Project", "value": list("MedScale CI Project")})
    click(session, button(session, "Create project"))
    print("MAC_NATIVE_WEBDRIVER_PROJECT_CREATE_ACTION")

    click(session, button(session, "Audio"))
    button(session, "Import synthetic tone sample")
    print("MAC_NATIVE_WEBDRIVER_AUDIO_CONTROLS_PASS")

    # A CI-only network failure is injected into the real native model HTTP
    # entry point before it can send bytes. This exercises the shipped GUI,
    # native worker and cleanup/error state without any model download.
    if os.getenv("MEDSCALE_MAC_E2E_FORCE_OFFLINE") == "1":
        if os.getenv("MEDSCALE_MAC_E2E_MODEL_GUI_DOWNLOAD") == "1" or os.getenv("MEDSCALE_E2E_REAL_WAV_PATH"):
            raise AssertionError("Offline negative proof cannot run with real model or audio sources")
        click(session, button(session, "Check disk budget"))
        await_element(session, "css selector", '[data-testid="audio-model-preflight"]')
        preflight = http("POST", f"/session/{session}/execute/sync", {
            "script": """return document.querySelector('[data-testid="audio-model-preflight"]')?.textContent?.includes('Enough local storage') === true;""",
            "args": [],
        })
        if preflight is not True:
            raise AssertionError("Host lacks sufficient free storage to test network failure; no download attempted")
        install = button(session, "Download, verify and install")
        if http("GET", f"/session/{session}/element/{install}/enabled") is not False:
            raise AssertionError("Model download must require consent before offline error test")
        print("MAC_NATIVE_OFFLINE_GUI_CONSENT_REQUIRED_PASS")
        consent = await_element(session, "css selector",
                                'input[aria-label="Approve Whisper model download"]')
        click(session, consent)
        if http("GET", f"/session/{session}/element/{install}/enabled") is not True:
            raise AssertionError("Checked storage and approval did not enable selected model download")
        click(session, install)
        failed_cleanly = False
        end = time.monotonic() + 55
        while time.monotonic() < end:
            observation = http("POST", f"/session/{session}/execute/sync", {
                "script": """const status=[...document.querySelectorAll('[role="status"]')].map(el=>el.textContent||'');return {failed:status.some(t=>t.includes('Model download: error.')&&t.includes('Network request failed; check connection and retry')&&t.includes('0 MiB received')),installed:document.body.innerText.includes('Local Whisper engine installed')};""",
                "args": [],
            })
            if observation.get("installed"):
                raise AssertionError("Model was unexpectedly installed in a no-network negative test")
            if observation.get("failed"):
                failed_cleanly = True
                break
            time.sleep(2)
        if not failed_cleanly:
            raise AssertionError("Actual native download worker did not expose a sanitized offline error in WKWebView")

        # The injected native acquisition path writes this path marker only
        # into RUNNER_TEMP. Inspect the actual cache root, never an inferred
        # application directory. No staging or verified Pack may survive.
        marker = Path(os.environ["RUNNER_TEMP"]) / "medscale-offline-cache-root.txt"
        if not marker.is_file():
            raise AssertionError("Native worker was not reached: missing CI cache-root proof")
        model_root = Path(marker.read_text().strip())
        if model_root.name != "audio-models" or Path.home() not in model_root.parents:
            raise AssertionError("Native worker reported an unexpected cache root")
        if not model_root.is_dir():
            raise AssertionError("Native model staging directory was not created")
        if (model_root / "whisper-tiny-en").exists():
            raise AssertionError("Unverified model was published after offline failure")
        if any(p.name.startswith(".download-") for p in model_root.iterdir()):
            raise AssertionError("Offline failure left an app-owned temporary model stage")
        print("MAC_NATIVE_OFFLINE_ERROR_AND_OWNED_STAGE_CLEANUP_PASS")

        if http("GET", f"/session/{session}/element/{install}/enabled") is not False:
            raise AssertionError("Failed download unexpectedly retained previous user consent")
        click(session, consent)
        if http("GET", f"/session/{session}/element/{install}/enabled") is not True:
            raise AssertionError("User cannot retry failed offline download after a new explicit approval")
        print("MAC_NATIVE_OFFLINE_RECONSENT_RETRY_READY_PASS")

    # This runs only on an ephemeral GitHub macOS runner and never selects a
    # file. It must observe an actual new OS window/sheet, then cancel it
    # through macOS accessibility rather than mocking the Tauri dialog API.
    if os.getenv("MEDSCALE_MAC_E2E_NATIVE_PICKER_CANCEL") == "1":
        def picker_windows_and_sheets():
            apple = f"""
tell application "System Events"
    set targetProcess to first application process whose unix id is {process.pid}
    set windowCount to count of windows of targetProcess
    set sheetCount to 0
    if windowCount > 0 then set sheetCount to count of sheets of window 1 of targetProcess
    return (windowCount as text) & "," & (sheetCount as text)
end tell
"""
            result = subprocess.run(["osascript", "-e", apple],
                                    capture_output=True, text=True, timeout=12)
            if result.returncode != 0:
                raise AssertionError("macOS Accessibility permission or native window inspection unavailable: "
                                     + result.stderr.strip()[:200])
            parts = result.stdout.strip().split(",")
            if len(parts) != 2 or not all(x.strip().isdigit() for x in parts):
                raise AssertionError("Unexpected macOS window/sheet count from System Events")
            return tuple(int(x.strip()) for x in parts)

        def send_targeted_escape():
            # Use the launched app PID, not whichever application happens to
            # be frontmost on the disposable macOS runner.
            escaped = subprocess.run(
                ["osascript", "-e",
                 f'''tell application "System Events"
    set targetProcess to first application process whose unix id is {process.pid}
    set frontmost of targetProcess to true
    key code 53
end tell'''],
                capture_output=True, text=True, timeout=12,
            )
            if escaped.returncode != 0:
                raise AssertionError("Native dialog cancellation requires macOS Accessibility approval: "
                                     + escaped.stderr.strip()[:200])

        def field_value(selector):
            return http("POST", f"/session/{session}/execute/sync", {
                "script": "return document.querySelector(arguments[0])?.value ?? null;",
                "args": [selector],
            })

        for selector, label, sentinel in [
            ("#wp1", "Browse WAV", "unchanged-private-audio.wav"),
            ("#ap1", "Browse folder", "unchanged-private-pack"),
        ]:
            # The sentinel denotes a non-existing path inside disposable CI;
            # it is never opened or imported, and no user source is touched.
            expected = str(Path(os.environ["RUNNER_TEMP"]) / sentinel)
            element = await_element(session, "css selector", selector)
            http("POST", f"/session/{session}/element/{element}/value",
                 {"text": expected, "value": list(expected)})
            if field_value(selector) != expected:
                raise AssertionError(f"Could not seed controlled picker input: {selector}")
            baseline = picker_windows_and_sheets()
            click(session, button(session, label))
            observed = False
            try:
                for _ in range(15):
                    current = picker_windows_and_sheets()
                    if current[0] > baseline[0] or current[1] > baseline[1]:
                        observed = True
                        break
                    time.sleep(1)
            finally:
                # Fail instead of claiming qualification if macOS denies
                # automation's Escape key: no fake cancellation success.
                send_targeted_escape()
            if not observed:
                raise AssertionError(f"No native file/folder dialog observed: {label}")
            # A successful Escape keystroke alone does not prove the native
            # AppKit chooser closed. Require the observed extra window/sheet
            # to disappear before asserting UI recovery or path retention.
            dismissed = False
            last_count = None
            escape_attempts = 1
            # Native folder panels can consume the first Escape inside a
            # focused subcontrol without closing the enclosing AppKit sheet.
            # Observe that explicitly and allow one more *real* OS Escape,
            # never an injected JavaScript/mocked dialog cancellation.
            for attempt in range(2):
                for _ in range(8 if attempt == 0 else 20):
                    current = picker_windows_and_sheets()
                    last_count = current
                    if current[0] <= baseline[0] and current[1] <= baseline[1]:
                        dismissed = True
                        break
                    time.sleep(1)
                if dismissed:
                    break
                if attempt == 0:
                    print("MAC_NATIVE_PICKER_FIRST_ESCAPE_UNDISMISSED", label,
                          f"baseline={baseline}", f"remaining={last_count}")
                    send_targeted_escape()
                    escape_attempts = 2
            if not dismissed:
                raise AssertionError(f"Native file/folder dialog remained open after {escape_attempts} targeted Escape key(s): {label}; baseline={baseline}, observed={current}, final={last_count}")
            print("MAC_NATIVE_PICKER_ESCAPE_ATTEMPTS", label, escape_attempts)
            print("MAC_NATIVE_PICKER_OS_DIALOG_DISMISSED_PASS", label)
            recovered = False
            for _ in range(20):
                if field_value(selector) == expected:
                    control = button(session, label)
                    enabled = http("GET", f"/session/{session}/element/{control}/enabled")
                    if enabled is True:
                        recovered = True
                        break
                time.sleep(1)
            if not recovered:
                raise AssertionError(f"Cancelled native picker changed path or left UI busy: {label}")
            print("MAC_NATIVE_PICKER_CANCEL_PRESERVES_PATH_PASS", label)

    # Optional explicit real-human test. This branch's dedicated workflow
    # provides *both* a legally sourced, verified real WAV and a pinned,
    # MedScale-signed local Whisper Pack in the disposable runner only.
    wav_path = os.environ.get("MEDSCALE_E2E_REAL_WAV_PATH")
    pack_path = os.environ.get("MEDSCALE_E2E_REAL_ASR_PACK_PATH")
    gui_model_download = os.environ.get("MEDSCALE_MAC_E2E_MODEL_GUI_DOWNLOAD") == "1"
    if wav_path or pack_path or gui_model_download:
        if not wav_path or not (pack_path or gui_model_download):
            raise SystemExit("Real ASR requires genuine WAV and either verified Pack or explicit GUI model download")
        temp = Path(os.environ["RUNNER_TEMP"]).resolve()
        sources = [("WAV", wav_path)] + ([("Pack", pack_path)] if pack_path else [])
        for name, source in sources:
            p = Path(source).resolve()
            if temp not in p.parents or not p.exists():
                raise SystemExit(f"Refusing {name} outside disposable runner temp")
        print("MAC_NATIVE_WEBDRIVER_REAL_AUDIO_INPUTS_VERIFIED")

        def fill(css, value):
            el = await_element(session, "css selector", css)
            if http("GET", f"/session/{session}/element/{el}/displayed") is not True:
                raise AssertionError(f"Expected visible form input: {css}")
            http("POST", f"/session/{session}/element/{el}/value",
                 {"text": str(value), "value": list(str(value))})

        if gui_model_download:
            # Verify a brand-new user's non-download preflight, explicit source
            # consent and actual native Core acquisition. Never install anything
            # on the founder's Mac/PC; this test runs only on GitHub's macOS runner.
            click(session, button(session, "Check disk budget"))
            await_element(session, "css selector", '[data-testid="audio-model-preflight"]')
            sufficient = http("POST", f"/session/{session}/execute/sync", {
                "script": """return document.querySelector('[data-testid="audio-model-preflight"]')?.textContent?.includes('Enough local storage') === true;""",
                "args": [],
            })
            if sufficient is not True:
                raise AssertionError("Real app rejected model disk preflight; no download attempted")
            print("MAC_NATIVE_WEBDRIVER_MODEL_DISK_PREFLIGHT_PASS")

            install = button(session, "Download, verify and install")
            if http("GET", f"/session/{session}/element/{install}/enabled") is not False:
                raise AssertionError("Model download must be disabled before explicit user consent")
            print("MAC_NATIVE_WEBDRIVER_DOWNLOAD_DISABLED_BEFORE_CONSENT_PASS")

            consent = await_element(session, "css selector",
                                    'input[aria-label="Approve Whisper model download"]')
            click(session, consent)
            if http("GET", f"/session/{session}/element/{install}/enabled") is not True:
                raise AssertionError("Explicit user consent did not enable model download")
            click(session, install)
            await_element(session, "xpath",
                          "//*[contains(text(),'Local Whisper engine installed')]", timeout=650)
            print("MAC_NATIVE_WEBDRIVER_CONSENTED_GUI_WHISPER_INSTALL_PASS")
        else:
            fill("#ap1", pack_path)
            click(session, button(session, "Admit and install"))
            await_element(session, "xpath",
                          "//*[contains(text(),'Local Whisper engine installed')]", timeout=110)
            print("MAC_NATIVE_WEBDRIVER_WHISPER_ADMITTED_PASS")

        fill("#wp1", wav_path)
        # Exact button: prevent accidentally clicking the synthetic importer.
        imported = await_element(session, "xpath", "//button[normalize-space(.)='Import']")
        click(session, imported)
        source = await_element(session, "xpath",
                               "//button[normalize-space(.)='Transcribe']", timeout=100)
        route = http("POST", f"/session/{session}/execute/sync", {
            "script": "return document.querySelector('select[aria-label=\"Transcription route\"]')?.value === 'local_asr_pack' && document.querySelector('select[aria-label=\"Transcription language\"]')?.value === 'en'",
            "args": [],
        })
        if route is not True:
            raise AssertionError("Native UI is not set to actual English local Whisper")
        print("MAC_NATIVE_WEBDRIVER_REAL_WAV_IMPORTED_PASS")

        click(session, source)
        verified = False
        deadline = time.monotonic() + 220
        # Only a boolean is returned/logged. No transcript, speech, or
        # patient data is printed or kept in public CI artifacts.
        while time.monotonic() < deadline:
            state = http("POST", f"/session/{session}/execute/sync", {
                "script": """const section=[...document.querySelectorAll('section.section')].find(x=>[...x.querySelectorAll('.sec-t')].some(h=>h.textContent?.trim()==='Transcript'));if(!section)return false;const corrected=[...section.querySelectorAll('button')].some(b=>b.textContent?.trim()==='Correct');return corrected&&!section.textContent.includes('(fixture)')&&!section.textContent.includes('(untranscribed)');""",
                "args": [],
            })
            if state is True:
                verified = True
                break
            time.sleep(3)
        if not verified:
            raise AssertionError("No confirmed, nonfixture real voice transcript visible in WKWebView")
        print("MAC_NATIVE_WEBDRIVER_REAL_HUMAN_WHISPER_TRANSCRIPT_PASS")
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