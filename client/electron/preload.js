const { contextBridge, ipcRenderer } = require("electron");

contextBridge.exposeInMainWorld("electron", {
    ipcRenderer: {
        on: (channel, func) => {
            ipcRenderer.on(channel, (event, ...args) => func(...args));
        },
    },
    hideAndClearWindow: () => {
        ipcRenderer.send("hide-and-clear-window");
    },
    searchClipboard: (query) => {
        ipcRenderer.send("search-clipboard", query);
    },
    getClipboardContent: () => {
        return new Promise((resolve) => {
            ipcRenderer.once("list-items", (event, items) => {
                resolve(items);
            });
            ipcRenderer.send("get-clipboard-content"); // 主进程需要响应这个事件
        });
    },
});
