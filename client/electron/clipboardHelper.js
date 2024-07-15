// clipboardHelper.js
const nativeBinding = require("/Users/zeke/work/py_work/HostClipboard/core/rs_core/hello.darwin-x64.node");

let clipboardHelper;

async function initializeClipboardHelper() {
    const dbPath = "/Users/zeke/work/workspace/wb_work/hello/test.db";
    clipboardHelper = await nativeBinding.JsClipboardHelper.new(dbPath);
}

async function getClipboardContent() {
    const result = [];
    const clipboardList = await clipboardHelper.getNumClipboardEntries(5);
    console.log(clipboardList);
    const sortedEntries = clipboardList.entries.sort(
        (a, b) => b.timestamp - a.timestamp,
    );

    // 提取 content
    const contentList = sortedEntries.map((item) => item.content);
    console.log(contentList);

    return contentList;
}

module.exports = {
    initializeClipboardHelper,
    getClipboardContent,
};
