/** What changed, by installed version, shown on Help & Feedback. The full history is
 * in CHANGELOG.md. Keep each line short, and describe only shipped behaviour. */
export const RELEASE_NOTES: Record<string, readonly string[]> = {
  "0.1.0": [
    "Install from medscale.si without an account or app store.",
    "Encrypted workspaces with recovery codes, crash recovery and a recovery screen in Settings.",
    "Import FHIR data, explore evidence, and run local models from the catalog.",
    "Download a Whisper model with your approval, then transcribe recordings on this computer.",
    "Screen reader labels for rows and navigation, checked with NVDA.",
    "Help & Feedback: report bugs, request features and check for updates from inside the app.",
  ],
};
