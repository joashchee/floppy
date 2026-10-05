/**
 * The host platform, for wording and file pickers that differ by OS
 * (docs/platform-parity.md). Read from the webview's user agent: WebKit on
 * macOS says "Macintosh", WebKitGTK on Linux says "Linux".
 */
export const isLinux = /Linux/.test(navigator.userAgent) && !/Android/.test(navigator.userAgent);
export const isWindows = /Windows/.test(navigator.userAgent);

/** The label for revealing a file in the system's file manager. */
export const SHOW_IN_FILES = isLinux ? "Show in Folder" : isWindows ? "Show in File Explorer" : "Show in Finder";
