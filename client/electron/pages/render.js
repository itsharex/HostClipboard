const textInput = document.getElementById("text-input");
const selectableList = document.getElementById("selectable-list");
const displayContainer = document.getElementById("display-container");

let selectedIndex = -1;
let isKeyboardSelection = false;
// 监听主进程发送的数据
window.electron.ipcRenderer.on("list-items", (listItems) => {
    // 清空现有的列表项
    selectableList.innerHTML = "";
    selectedIndex = -1; // Reset selected index each time list is updated

    // 使用 listItems 来填充列表
    listItems.forEach((item, index) => {
        const li = document.createElement("li");
        li.textContent = item;
        li.addEventListener("click", () => {
            displayContainer.textContent = `You selected: ${item}`;
            selectedIndex = index;
            updateSelectedItem();
            copyToClipboardAndHide(item);
        });
        li.addEventListener("mouseover", () => {
            if (!isKeyboardSelection) {
                selectedIndex = index;
                updateSelectedItem();
            }
        });
        li.addEventListener("mouseout", () => {
            if (!isKeyboardSelection) {
                li.classList.remove("selected");
            }
        });

        selectableList.appendChild(li);
    });
    updateSelectedItem(); // Update the UI to reflect no selection
});

// 处理输入框的输入
textInput.addEventListener("input", async (e) => {
    const inputValue = e.target.value;
    displayContainer.textContent = `${inputValue}`;
    if (inputValue.trim() !== "") {
        window.electron.searchClipboard(inputValue);
    } else {
        const listItems = await window.electron.getClipboardContent(); // 这需要在 preload.js 中暴露新的方法
        updateList(listItems);
    }
});

// 更新选中的列表项和右侧的显示内容
function updateSelectedItem() {
    const items = selectableList.querySelectorAll("li");
    items.forEach((item, index) => {
        if (index === selectedIndex) {
            item.classList.add("selected");
            displayContainer.textContent = `${item.textContent}`;
        } else {
            item.classList.remove("selected");
        }
    });
}

// 复制内容到剪切板并隐藏窗口
function copyToClipboardAndHide(content) {
    navigator.clipboard
        .writeText(content)
        .then(() => {
            console.log("Content copied to clipboard");
            window.electron.hideAndClearWindow();
        })
        .catch((err) => {
            console.error("Failed to copy text: ", err);
        });
}

// 监听键盘事件
document.addEventListener("keydown", (e) => {
    const items = selectableList.querySelectorAll("li");
    if (e.key === "ArrowUp" || e.key === "ArrowDown") {
        e.preventDefault();
        isKeyboardSelection = true;
        document.body.style.cursor = "none";

        if (e.key === "ArrowUp" && selectedIndex > 0) {
            selectedIndex--;
        } else if (e.key === "ArrowDown" && selectedIndex < items.length - 1) {
            selectedIndex++;
        }

        updateSelectedItem();
        if (selectedIndex >= 0 && selectedIndex < items.length) {
            items[selectedIndex].scrollIntoView({
                // behavior: "smooth", // 平滑滚动
                block: "nearest", // 最近边界，减少不必要的滚动
            });
        }
    } else if (e.key === "Enter") {
        if (selectedIndex !== -1) {
            const selectedItem = items[selectedIndex].textContent;
            copyToClipboardAndHide(selectedItem);
        }
    }
});

// 监听鼠标事件
document.addEventListener("mousemove", () => {
    isKeyboardSelection = false;
    document.body.style.cursor = "auto";
});

updateSelectedItem();
