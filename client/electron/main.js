const {
    app, BrowserWindow, Tray, Menu, globalShortcut, ipcMain,
} = require("electron");
const path = require("path");

let mainWindow;
let tray = null;

function hideAndClearWindow() {
    if (mainWindow) {
        mainWindow.hide();
        mainWindow.webContents.executeJavaScript('document.getElementById("text-input").value = ""'); // 清空输入框
    }
}

function createWindow() {
    mainWindow = new BrowserWindow({
        width: 800, height: 600, webPreferences: {
            preload: path.join(__dirname, "preload.js"), nodeIntegration: false, // 禁用 Node.js 集成
            contextIsolation: true, // 启用上下文隔离
        }, // show: false, // 隐藏窗口
        frame: false, // 创建无边框窗口
    });

    mainWindow.loadFile("pages/index.html");
    // 发送数据到渲染进程
    mainWindow.webContents.on("did-finish-load", () => {
        mainWindow.webContents.send("list-items", ["Item 1", "Item 2", "Item 3"]);
    });

    if (process.platform === 'darwin') {
        app.dock.hide();
    }

    // 创建系统托盘图标
    if (!tray) {
        tray = new Tray(path.join(__dirname, "icons/bar_18x18.png")); // 替换为你的图标路径
        const contextMenu = Menu.buildFromTemplate([{
            label: "Show App", click: function () {
                if (mainWindow) {
                    mainWindow.show();
                    mainWindow.webContents.executeJavaScript('document.getElementById("text-input").focus()');
                } else {
                    createWindow();
                }
            },
        }, {
            label: "Quit", click: function () {
                app.isQuitting = true;
                app.quit();
            },
        },]);
        tray.setContextMenu(contextMenu);
        tray.setToolTip("This is my application.");
    }

    // 监听窗口关闭事件，隐藏窗口而不是销毁它
    mainWindow.on("close", (event) => {
        if (!app.isQuitting) {
            event.preventDefault();
            hideAndClearWindow();
        }
    });

    mainWindow.on("closed", () => {
        mainWindow = null;
    });

    mainWindow.on("blur", () => {
        hideAndClearWindow();
    });


}

app.whenReady().then(() => {
    createWindow();

    // 注册全局快捷键
    globalShortcut.register("CommandOrControl+Shift+L", () => {
        if (mainWindow) {
            if (mainWindow.isVisible()) {
                hideAndClearWindow();
            } else {
                mainWindow.show();
                mainWindow.webContents.executeJavaScript('document.getElementById("text-input").focus()');
            }
        } else {
            createWindow();
        }
    });

    // 监听渲染进程发送的 hide-and-clear-window 事件
    ipcMain.on("hide-and-clear-window", () => {
        hideAndClearWindow();
    });

    app.on("activate", function () {
        if (BrowserWindow.getAllWindows().length === 0) createWindow();
    });
});

app.on("window-all-closed", function () {
    if (process.platform !== "darwin") app.quit();
});

app.on("will-quit", () => {
    // 注销所有快捷键
    globalShortcut.unregisterAll();
});
