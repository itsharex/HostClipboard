<script setup lang="ts">
import { ref, onMounted, computed, watch } from "vue";
import { appWindow } from "@tauri-apps/api/window";
import { ClipboardHelper, ClipboardEntry } from "../clipboardHelper";
import { invoke } from "@tauri-apps/api/tauri";



const textInput = ref("");
const clipboardEntries = ref<ClipboardEntry[]>([]);
const selectedIndex = ref(-1);
let isKeyboardSelection = ref(true);

function openSettings() {
    invoke("open_settings");
}


const displayContent = computed(() => {
    if (selectedIndex.value >= 0 && selectedIndex.value < clipboardEntries.value.length) {
        return clipboardEntries.value[selectedIndex.value].content;
    }
    return "";
});

async function getClipboardContent() {
    try {
        clipboardEntries.value = await ClipboardHelper.getClipboardEntries();
        selectedIndex.value = -1; // Reset selection
    } catch (error) {
        console.error("Failed to get clipboard content:", error);
        clipboardEntries.value = [];
    }
}

async function searchClipboard() {
    try {
        clipboardEntries.value = await ClipboardHelper.searchClipboardEntries(textInput.value);
        selectedIndex.value = -1; // Reset selection
    } catch (error) {
        console.error("Failed to search clipboard content:", error);
        clipboardEntries.value = [];
    }
}

async function copyToClipboardAndHide(content: string) {
    try {
        await navigator.clipboard.writeText(content);
        console.log("Content copied to clipboard");
        await appWindow.hide();
    } catch (err) {
        console.error("Failed to copy text or hide window: ", err);
    }
}

function handleKeydown(e: KeyboardEvent) {
    if (e.key === "ArrowUp" || e.key === "ArrowDown") {
        e.preventDefault();
        isKeyboardSelection.value = true;
        if (e.key === "ArrowUp" && selectedIndex.value > 0) {
            selectedIndex.value--;
        } else if (
            e.key === "ArrowDown" &&
            selectedIndex.value < clipboardEntries.value.length - 1
        ) {
            selectedIndex.value++;
        }
    } else if (e.key === "Enter") {
        if (selectedIndex.value !== -1) {
            const selectedItem = clipboardEntries.value[selectedIndex.value];
            copyToClipboardAndHide(selectedItem.content);
        }
    } else if (e.key === "Escape") {
        appWindow.hide();
    } else if ((e.metaKey || e.ctrlKey) && e.key === ',') {
        e.preventDefault() // 阻止默认行为
        try {
            openSettings()
        } catch (e) {
            console.error('Failed to open settings:', e)
        }
    }
}

function handleMouseMove() {
    isKeyboardSelection.value = false;
}

const inputRef = ref<HTMLInputElement | null>(null);

onMounted(async () => {
    await getClipboardContent();
    document.addEventListener("keydown", handleKeydown);
    document.addEventListener("mousemove", handleMouseMove);


    await appWindow.onFocusChanged(({ payload: focused }) => {
        if (focused) {
            textInput.value = "";
            getClipboardContent();
            inputRef.value?.focus();
        }
    });

    inputRef.value?.focus();
});

watch(textInput, () => {
    if (textInput.value.trim() !== "") {
        searchClipboard();
    } else {
        getClipboardContent();
    }
});


const selectedTimestamp = computed(() => {
    if (selectedIndex.value >= 0 && selectedIndex.value < clipboardEntries.value.length) {
        return clipboardEntries.value[selectedIndex.value].timestamp;
    }
    return null;
});

const formattedTimestamp = computed(() => {
    if (selectedTimestamp.value) {
        // 将秒转换为毫秒
        const milliseconds = selectedTimestamp.value * 1000;
        const date = new Date(milliseconds);

        // 使用更易读的格式
        const options: Intl.DateTimeFormatOptions = {
            year: 'numeric',
            month: '2-digit',
            day: '2-digit',
            hour: '2-digit',
            minute: '2-digit',
            second: '2-digit',
            hour12: false
        };

        return date.toLocaleString(undefined, options);
    }
    return "";
});

</script>

<template>
    <div id="app">
        <div id="input-container">
            <input ref="inputRef" v-model="textInput" type="text" id="text-input" placeholder="Enter text..." />
        </div>
        <div id="content-container">
            <div id="list-container">
                <ul id="selectable-list">
                    <li v-for="(item, index) in clipboardEntries" :key="item.id"
                        :class="{ selected: index === selectedIndex }" @click="() => {
                            selectedIndex = index;
                            copyToClipboardAndHide(item.content);
                        }" @mouseover="() => {
                            if (!isKeyboardSelection) {
                                selectedIndex = index;
                            }
                        }">
                        {{ item.content }}
                    </li>
                </ul>
            </div>
            <div id="display-container">
                <div class="content-wrapper">
                    <pre>{{ displayContent }}</pre>
                </div>
                <div class="timestamp-wrapper">
                    <div class="timestamp" v-if="selectedTimestamp">{{ formattedTimestamp }}</div>
                    <button @click="openSettings" class="settings-button">⚙️</button>
                </div>
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
    font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, "Helvetica Neue", Arial, sans-serif;
    overflow: hidden;
    /* 防止内容超出圆角区域 */
}

#app {
    display: flex;
    flex-direction: column;
    height: 100vh;
    border-radius: 12px;
    overflow: hidden;
    background-color: #ffffff;
    /* 或者您想要的背景色 */
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
    border-radius: 12px;
    /* 圆角效果 */
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
    border-radius: 8px;
    /* 圆角效果 */
}

#selectable-list li.selected {
    background-color: #4caf50;
    color: white;
    border-radius: 8px;
    /* 圆角效果 */
}

#display-container {
    width: 50%;
    height: 100%;
    display: flex;
    flex-direction: column;
    position: relative;
}

.content-wrapper {
    flex-grow: 1;
    overflow-y: auto;
    padding: 10px;
    padding-bottom: 40px;
    /* 为时间戳留出空间 */
    border-radius: 12px;
    /* 圆角效果 */
}

.content-wrapper pre {
    font-size: 13.5px;
    font-family: system-ui, sans-serif;
    white-space: pre-wrap;
    word-wrap: break-word;
    margin: 0;
    line-height: 1.4;
}

.timestamp-wrapper {
    height: 30px;
    width: 100%;
    display: flex;
    align-items: center;
    justify-content: center;
    /* Center the timestamp */
    background-color: #f0f0f0;
    border-top: 1px solid #ccc;
    position: absolute;
    bottom: 0;
    left: 0;
    padding: 0 10px;
    box-sizing: border-box;
}

.timestamp {
    font-size: 15px;
    color: #666;
    padding: 2px 5px;
    border-radius: 8px;
    /* 圆角效果 */
    text-align: center;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
}

.settings-button {
    padding: 5px 10px;
    background-color: transparent;
    color: #4CAF50;
    border: none;
    border-radius: 8px;
    /* 圆角效果 */
    cursor: pointer;
    font-size: 18px;
    position: absolute;
    right: 0.3%;
    /* 固定距离右边缘 */
    bottom: 50%;
    /* 在包装中垂直居中 */
    transform: translateY(50%);
    /* 调整垂直定心 */
}

.settings-button:hover {
    background-color: rgba(28, 57, 29, 0.1);
    border-radius: 12px;
    /* 圆角效果 */
}
</style>
