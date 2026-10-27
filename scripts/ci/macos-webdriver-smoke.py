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

    # Optional explicit real-human test. This branch's dedicated workflow
    # provides *both* a legally sourced, verified real WAV and a pinned,
    # MedScale-signed local Whisper Pack in the disposable runner only.
    wav_path = os.environ.get("MEDSCALE_E2E_REAL_WAV_PATH")
    pack_path = os.environ.get("MEDSCALE_E2E_REAL_ASR_PACK_PATH")
    if wav_path or pack_path:
        if not wav_path or not pack_path:
            raise SystemExit("Real ASR qualification requires both actual WAV and verified Pack")
        temp = Path(os.environ["RUNNER_TEMP"]).resolve()
        for name, source in (("WAV", wav_path), ("Pack", pack_path)):
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