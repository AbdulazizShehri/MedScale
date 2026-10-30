/** Fail closed on corrupted or untrusted WebView preference strings. */
export type UiTheme = "light" | "dark";
export type UiDensity = "standard" | "compact";

export function parseTheme(value: string | null): UiTheme | null {
  return value === "light" || value === "dark" ? value : null;
}

export function parseDensity(value: string | null): UiDensity | null {
  return value === "standard" || value === "compact" ? value : null;
}
