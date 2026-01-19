import { invoke } from "@tauri-apps/api/core";
import { check, type Update } from "@tauri-apps/plugin-updater";
import { relaunch } from "@tauri-apps/plugin-process";

const view = new URLSearchParams(window.location.search).get("view") ?? "input";

document.body.dataset.view = view;

if (view === "titlebar") {
  const titlebar = document.querySelector(".titlebar") as HTMLElement | null;
  titlebar?.setAttribute("data-tauri-drag-region", "true");

  const updateIndicator = document.getElementById("update-indicator");
  const updateBtn = document.getElementById("update-btn");
  const updateProgress = document.getElementById("update-progress");
  const progressBar = document.getElementById("progress-bar");

  async function checkForUpdates() {
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

  async function downloadAndInstall(update: Update) {
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

  setTimeout(checkForUpdates, 3000);

  // Refresh Gemini session every 3 minutes to maintain WebView detection bypass
  setInterval(() => {
    invoke("refresh_gemini_session").catch(() => {});
  }, 3 * 60 * 1000);
} else {
  const input = document.getElementById("unified-input") as HTMLTextAreaElement;
  const sendBtn = document.getElementById("send-btn") as HTMLButtonElement;
  const inputBar = document.querySelector(".input-bar") as HTMLElement;
  const inputShell = document.querySelector(".input-shell") as HTMLElement;

  const MIN_TEXTAREA_HEIGHT = 40;
  const MAX_TEXTAREA_HEIGHT = 340;

  let lastInputBarHeight = 0;

  function getVerticalExtras(element: HTMLElement) {
    const styles = getComputedStyle(element);
    return (
      parseFloat(styles.paddingTop) +
      parseFloat(styles.paddingBottom) +
      parseFloat(styles.borderTopWidth) +
      parseFloat(styles.borderBottomWidth)
    );
  }

  function updateInputBarHeight(textareaHeight: number) {
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

  async function sendToAll() {
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

  function resizeTextarea() {
    input.style.height = "auto";
    const rawHeight = input.scrollHeight;
    const nextHeight = Math.min(
      Math.max(rawHeight, MIN_TEXTAREA_HEIGHT),
      MAX_TEXTAREA_HEIGHT,
    );
    input.style.height = nextHeight + "px";
    input.style.overflowY = "hidden";
    input.scrollTop = 0;

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
}
