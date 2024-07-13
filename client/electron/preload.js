const { contextBridge, ipcRenderer } = require('electron');

contextBridge.exposeInMainWorld('electron', {
  ipcRenderer: {
    on: (channel, func) => {
      ipcRenderer.on(channel, (event, ...args) => func(...args));
    }
  },
  hideAndClearWindow: () => {
    ipcRenderer.send('hide-and-clear-window');
  }
});
