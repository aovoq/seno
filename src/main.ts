import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import {
  isPermissionGranted,
  requestPermission,
  sendNotification,
} from "@tauri-apps/plugin-notification";
import { relaunch } from "@tauri-apps/plugin-process";
import { check, type Update } from "@tauri-apps/plugin-updater";

const view = new URLSearchParams(window.location.search).get("view") ?? "input";

document.body.dataset.view = view;

if (view === "titlebar") {
  const titlebar = document.querySelector(".titlebar") as HTMLElement | null;
  titlebar?.setAttribute("data-tauri-drag-region", "true");

  const updateIndicator = document.getElementById("update-indicator");
  const updateBtn = document.getElementById("update-btn");
  const updateProgress = document.getElementById("update-progress");
  const progressBar = document.getElementById("progress-bar");
  const toastIndicator = document.getElementById("provider-toast");
  const memoryIndicator = document.getElementById("memory-indicator");

  const statusItems = {
    claude: document.querySelector('[data-provider="claude"]') as HTMLElement | null,
    chatgpt: document.querySelector('[data-provider="chatgpt"]') as HTMLElement | null,
    gemini: document.querySelector('[data-provider="gemini"]') as HTMLElement | null,
  };

  const statusTexts = {
    claude: document.getElementById("status-claude"),
    chatgpt: document.getElementById("status-chatgpt"),
    gemini: document.getElementById("status-gemini"),
  };

  // Track streaming status for all providers
  const providerStates: Record<string, string> = {
    claude: "unknown",
    chatgpt: "unknown",
    gemini: "unknown",
  };

  // Track if any provider was streaming (to detect completion)
  let wasAnyStreaming = false;

  function checkAllIdleAndNotify(): void {
    const allIdle = Object.values(providerStates).every(
      (state) => state === "idle"
    );

    if (allIdle && wasAnyStreaming) {
      wasAnyStreaming = false;
      sendCompletionNotification();
    }
  }

  async function sendCompletionNotification(): Promise<void> {
    try {
      let hasPermission = await isPermissionGranted();
      if (!hasPermission) {
        hasPermission = (await requestPermission()) === "granted";
      }
      if (!hasPermission) return;

      sendNotification({
        title: "Seno",
        body: "All AI responses completed",
      });
    } catch (e) {
      console.warn("Notification failed:", e);
    }
  }

  async function checkForUpdates(): Promise<void> {
    try {
      const update = await check();
      if (update && updateIndicator && updateBtn) {
        updateIndicator.style.display = "flex";
        updateBtn.addEventListener("click", () => downloadAndInstall(update));
      }
    } catch (e) {
      console.warn("Update check failed:", e);
    }
  }

  async function downloadAndInstall(update: Update): Promise<void> {
    if (!updateBtn || !updateProgress || !progressBar) return;
    updateBtn.classList.add("downloading");
    updateBtn.querySelector(".update-text")!.textContent = "Downloading...";
    updateProgress.style.display = "block";

    try {
      let downloaded = 0;
      let total = 0;
      await update.downloadAndInstall((event) => {
        if (event.event === "Started") total = event.data.contentLength ?? 0;
        if (event.event === "Progress") {
          downloaded += event.data.chunkLength;
          if (total > 0) progressBar.style.width = `${(downloaded / total) * 100}%`;
        }
      });
      await relaunch();
    } catch (e) {
      console.error("Update failed:", e);
      updateBtn.querySelector(".update-text")!.textContent = "Update failed";
    }
  }

  function setStatus(provider: keyof typeof statusItems, state: string): void {
    const item = statusItems[provider];
    const text = statusTexts[provider];
    if (!item || !text) return;

    const prevState = providerStates[provider];
    providerStates[provider] = state;
    item.dataset.state = state;

    switch (state) {
      case "streaming":
        wasAnyStreaming = true;
        text.textContent = "Streaming";
        break;
      case "idle":
        text.textContent = "Idle";
        if (prevState === "streaming") {
          checkAllIdleAndNotify();
        }
        break;
      default:
        text.textContent = "Unknown";
    }
  }

  setTimeout(checkForUpdates, 3000);

  function formatMemory(mb: number): string {
    if (mb >= 1024) {
      return `${(mb / 1024).toFixed(1)} GB`;
    }
    return `${Math.round(mb)} MB`;
  }

  async function refreshMemoryUsage(): Promise<void> {
    try {
      const memoryMb = await invoke<number>("get_memory_usage");
      if (memoryIndicator) {
        memoryIndicator.textContent = `Memory: ${formatMemory(memoryMb)}`;
      }
    } catch (e) {
      console.warn("Failed to get memory usage:", e);
    }
  }

  refreshMemoryUsage();
  setInterval(refreshMemoryUsage, 5000);

  // Refresh Gemini session every 90 seconds to maintain WebView detection bypass
  setInterval(() => {
    invoke("refresh_gemini_session").catch(() => {});
  }, 90 * 1000);

  function handleProviderStatus(event: { payload: { provider: string; status: string } }): void {
    const provider = event.payload.provider as keyof typeof statusItems;
    if (!provider || !(provider in statusItems)) return;
    setStatus(provider, event.payload.status);
  }

  listen<{ provider: string; status: string }>("provider-status", handleProviderStatus).catch((err) => {
    console.warn("Failed to listen provider status:", err);
  });

  let toastTimer: number | null = null;

  function handleProviderToast(event: { payload: { provider: string; message: string } }): void {
    if (!toastIndicator) return;
    const provider = event.payload.provider.toUpperCase();
    toastIndicator.textContent = `${provider}: ${event.payload.message}`;
    toastIndicator.style.display = "flex";
    if (toastTimer) window.clearTimeout(toastTimer);
    toastTimer = window.setTimeout(() => {
      toastIndicator.style.display = "none";
    }, 10000);
  }

  listen<{ provider: string; message: string }>("provider-toast", handleProviderToast).catch((err) => {
    console.warn("Failed to listen provider toast:", err);
  });
} else {
  const input = document.getElementById("unified-input") as HTMLTextAreaElement;
  const sendBtn = document.getElementById("send-btn") as HTMLButtonElement;
  const inputBar = document.querySelector(".input-bar") as HTMLElement;
  const inputShell = document.querySelector(".input-shell") as HTMLElement;

  const MIN_TEXTAREA_HEIGHT = 40;
  const MAX_TEXTAREA_HEIGHT = 340;

  let lastInputBarHeight = 0;

  function getVerticalExtras(element: HTMLElement): number {
    const styles = getComputedStyle(element);
    return (
      parseFloat(styles.paddingTop) +
      parseFloat(styles.paddingBottom) +
      parseFloat(styles.borderTopWidth) +
      parseFloat(styles.borderBottomWidth)
    );
  }

  function updateInputBarHeight(textareaHeight: number): void {
    const shellExtras = getVerticalExtras(inputShell);
    const barExtras = getVerticalExtras(inputBar);
    const shellHeight = textareaHeight + shellExtras;
    const buttonHeight = sendBtn.getBoundingClientRect().height;
    const nextHeight = Math.ceil(Math.max(shellHeight, buttonHeight) + barExtras);
    if (!Number.isFinite(nextHeight) || nextHeight <= 0) {
      return;
    }
    if (nextHeight === lastInputBarHeight) {
      return;
    }
    lastInputBarHeight = nextHeight;
    console.debug("update_input_height", nextHeight);
    invoke("update_input_height", { height: nextHeight }).catch((err) => {
      console.error("Failed to update input height:", err);
    });
  }

  async function sendToAll(): Promise<void> {
    const text = input.value.trim();
    if (!text) return;

    sendBtn.disabled = true;

    try {
      await invoke("send_to_all", { text });
      input.value = "";
      resizeTextarea();
    } catch (err) {
      console.error("Failed to send:", err);
    } finally {
      sendBtn.disabled = false;
    }
  }

  function resizeTextarea(): void {
    input.style.height = "auto";
    const rawHeight = input.scrollHeight;
    const nextHeight = Math.min(
      Math.max(rawHeight, MIN_TEXTAREA_HEIGHT),
      MAX_TEXTAREA_HEIGHT,
    );
    input.style.height = nextHeight + "px";

    if (rawHeight > MAX_TEXTAREA_HEIGHT) {
      input.style.overflowY = "auto";
    } else {
      input.style.overflowY = "hidden";
      input.scrollTop = 0;
    }

    updateInputBarHeight(nextHeight);
  }

  sendBtn.addEventListener("click", sendToAll);

  input.addEventListener("keydown", (e) => {
    if (e.key === "Enter" && e.metaKey) {
      e.preventDefault();
      sendToAll();
    }
  });

  input.addEventListener("input", resizeTextarea);

  window.addEventListener("resize", () => {
    updateInputBarHeight(input.getBoundingClientRect().height);
  });

  resizeTextarea();
  updateInputBarHeight(input.getBoundingClientRect().height);

  // Focus management
  async function focusInput(): Promise<void> {
    await invoke("focus_input");
    input.focus();
  }

  focusInput();
  window.addEventListener("focus", () => setTimeout(focusInput, 50));
}
