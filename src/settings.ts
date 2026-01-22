import { invoke } from "@tauri-apps/api/core";

interface TitlebarElement {
  id: string;
  visible: boolean;
}

interface DisplaySettings {
  elements: TitlebarElement[];
}

const ELEMENT_LABELS: Record<string, string> = {
  memory: "Memory",
  serviceStatus: "Status",
  geminiReinject: "Reinject",
  providerToast: "Toast",
};

let settings: DisplaySettings = {
  elements: [],
};

const listEl = document.getElementById("settings-list")!;

let draggedItem: HTMLElement | null = null;
let draggedIndex = -1;

async function loadSettings(): Promise<void> {
  try {
    settings = await invoke<DisplaySettings>("get_display_settings");
    renderList();
  } catch (e) {
    console.warn("[settings] Failed to load settings:", e);
  }
}

async function saveSettings(): Promise<void> {
  try {
    await invoke("set_display_settings", { settings });
  } catch (e) {
    console.warn("[settings] Failed to save settings:", e);
  }
}

function createItem(element: TitlebarElement, index: number): HTMLElement {
  const item = document.createElement("div");
  item.className = "settings-item";
  item.dataset.index = String(index);

  item.innerHTML = `
    <div class="drag-handle"><span></span><span></span></div>
    <span class="item-label">${ELEMENT_LABELS[element.id] || element.id}</span>
    <label class="toggle">
      <input type="checkbox" ${element.visible ? "checked" : ""} />
      <div class="toggle-track"></div>
      <div class="toggle-thumb"></div>
    </label>
  `;

  const checkbox = item.querySelector("input")!;
  checkbox.addEventListener("change", (e) => {
    e.stopPropagation();
    element.visible = checkbox.checked;
    saveSettings();
  });

  // Prevent drag when clicking toggle
  const toggle = item.querySelector(".toggle")!;
  toggle.addEventListener("mousedown", (e) => e.stopPropagation());

  item.addEventListener("mousedown", (e) => handleMouseDown(e, item, index));

  return item;
}

function renderList(): void {
  listEl.innerHTML = "";
  settings.elements.forEach((element, index) => {
    listEl.appendChild(createItem(element, index));
  });
}

function handleMouseDown(e: MouseEvent, item: HTMLElement, index: number): void {
  if ((e.target as HTMLElement).closest(".toggle")) return;

  e.preventDefault();
  draggedItem = item;
  draggedIndex = index;

  item.classList.add("dragging");

  document.addEventListener("mousemove", handleMouseMove);
  document.addEventListener("mouseup", handleMouseUp);
}

function handleMouseMove(e: MouseEvent): void {
  if (!draggedItem) return;

  const items = Array.from(listEl.querySelectorAll(".settings-item:not(.dragging)")) as HTMLElement[];

  for (const item of items) {
    const rect = item.getBoundingClientRect();
    const midY = rect.top + rect.height / 2;

    if (e.clientY < midY) {
      item.classList.add("drag-over");
      items.filter(i => i !== item).forEach(i => i.classList.remove("drag-over"));
      return;
    } else {
      item.classList.remove("drag-over");
    }
  }

  // If below all items, mark last one
  if (items.length > 0) {
    const lastItem = items[items.length - 1];
    lastItem.classList.add("drag-over-bottom");
    items.slice(0, -1).forEach(i => i.classList.remove("drag-over-bottom"));
  }
}

function handleMouseUp(): void {
  document.removeEventListener("mousemove", handleMouseMove);
  document.removeEventListener("mouseup", handleMouseUp);

  if (!draggedItem) return;

  const items = Array.from(listEl.querySelectorAll(".settings-item")) as HTMLElement[];
  const overItem = items.find(item =>
    item.classList.contains("drag-over") || item.classList.contains("drag-over-bottom")
  );

  if (overItem && overItem !== draggedItem) {
    const toIndex = parseInt(overItem.dataset.index!, 10);
    const isBottom = overItem.classList.contains("drag-over-bottom");

    const [moved] = settings.elements.splice(draggedIndex, 1);
    const insertIndex = isBottom ? toIndex + 1 : toIndex;
    const adjustedIndex = draggedIndex < toIndex ? insertIndex - 1 : insertIndex;
    settings.elements.splice(adjustedIndex < 0 ? 0 : adjustedIndex, 0, moved);

    saveSettings();
    renderList();
  } else {
    draggedItem.classList.remove("dragging");
  }

  // Clean up
  items.forEach(item => {
    item.classList.remove("drag-over", "drag-over-bottom");
  });

  draggedItem = null;
  draggedIndex = -1;
}

loadSettings();
