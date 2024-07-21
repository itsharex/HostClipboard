<script setup lang="ts">
import { ref, onMounted } from "vue";
import { invoke } from "@tauri-apps/api/tauri";
import { appWindow } from "@tauri-apps/api/window";

const textInput = ref("");
const selectableList = ref<string[]>([]);
const displayContent = ref("");
const selectedIndex = ref(-1);
let isKeyboardSelection = false;

async function getClipboardContent() {
  try {
    const items = await invoke("get_clipboard_content");
    selectableList.value = items as string[];
  } catch (error) {
    console.error("Failed to get clipboard content:", error);
  }
}

async function searchClipboard() {
  if (textInput.value.trim() !== "") {
    try {
      const items = await invoke("search_clipboard", {
        query: textInput.value,
      });
      selectableList.value = items as string[];
    } catch (error) {
      console.error("Failed to search clipboard:", error);
    }
  } else {
    await getClipboardContent();
  }
}

function updateSelectedItem() {
  if (
    selectedIndex.value >= 0 &&
    selectedIndex.value < selectableList.value.length
  ) {
    displayContent.value = selectableList.value[selectedIndex.value];
  } else {
    displayContent.value = "";
  }
}

async function copyToClipboardAndHide(content: string) {
  try {
    await navigator.clipboard.writeText(content);
    console.log("Content copied to clipboard");
    await invoke("toggle_window");
  } catch (err) {
    console.error("Failed to copy text or hide window: ", err);
  }
}

function handleKeydown(e: KeyboardEvent) {
  if (e.key === "ArrowUp" || e.key === "ArrowDown") {
    e.preventDefault();
    isKeyboardSelection = true;
    document.body.style.cursor = "none";
    if (e.key === "ArrowUp" && selectedIndex.value > 0) {
      selectedIndex.value--;
    } else if (
      e.key === "ArrowDown" &&
      selectedIndex.value < selectableList.value.length - 1
    ) {
      selectedIndex.value++;
    }
    updateSelectedItem();
  } else if (e.key === "Enter") {
    if (selectedIndex.value !== -1) {
      const selectedItem = selectableList.value[selectedIndex.value];
      copyToClipboardAndHide(selectedItem);
    }
  } else if (e.key === "Escape") {
    invoke("toggle_window");
  }
}

function handleMouseMove() {
  if (isKeyboardSelection) {
    isKeyboardSelection = false;
    document.body.style.cursor = "auto";
  }
}

const inputRef = ref<HTMLInputElement | null>(null);

onMounted(async () => {
  await getClipboardContent();
  document.addEventListener("keydown", handleKeydown);
  document.addEventListener("mousemove", handleMouseMove);

  // 监听窗口显示事件
  await appWindow.onFocusChanged(({ payload: focused }) => {
    if (focused) {
      textInput.value = "";
      getClipboardContent();
      // 聚焦输入框
      inputRef.value?.focus();
    }
  });

  // 初始聚焦
  inputRef.value?.focus();
});
</script>

<template>
  <div id="app">
    <div id="input-container">
      <input ref="inputRef" v-model="textInput" type="text" id="text-input" placeholder="Enter text..."
        @input="searchClipboard" />
    </div>
    <div id="content-container">
      <div id="list-container">
        <ul id="selectable-list">
          <li v-for="(item, index) in selectableList" :key="index" :class="{ selected: index === selectedIndex }"
            @click="() => {
              selectedIndex = index;
              updateSelectedItem();
              copyToClipboardAndHide(item);
            }
              " @mouseover="() => {
                if (!isKeyboardSelection) {
                  selectedIndex = index;
                  updateSelectedItem();
                }
              }
                ">
            {{ item }}
          </li>
        </ul>
      </div>
      <div id="display-container">
        {{ displayContent }}
      </div>
    </div>
  </div>
</template>

<style>
body,
html {
  margin: 0;
  padding: 0;
  height: 100%;
  font-family: Arial, sans-serif;
}

#app {
  display: flex;
  flex-direction: column;
  height: 100vh;
}

#input-container {
  padding: 10px;
  background-color: #f0f0f0;
  display: flex;
  justify-content: center;
}

#text-input {
  width: 100%;
  padding: 10px;
  font-size: 16px;
  border: 1px solid #ccc;
  border-radius: 4px;
}

#content-container {
  display: flex;
  flex: 1;
  overflow: hidden;
}

#list-container {
  width: 50%;
  overflow-y: auto;
  border-right: 1px solid #ccc;
}

#display-container {
  width: 50%;
  padding: 10px;
  overflow-y: auto;
}

#selectable-list {
  list-style-type: none;
  padding: 0;
  margin: 0;
}

#selectable-list li {
  padding: 10px;
  cursor: pointer;
  border-bottom: 1px solid #eee;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

#selectable-list li:hover,
#selectable-list li.selected {
  background-color: #4caf50;
  color: white;
}
</style>
