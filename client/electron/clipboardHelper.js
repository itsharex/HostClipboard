// clipboardHelper.js
const nativeBinding = require("/Users/zeke/work/workspace/github_work/HostClipboard/core/rs_core/hello.darwin-arm64.node");

let clipboardHelper;

async function initializeClipboardHelper() {
    const dbPath = "/Users/zeke/work/workspace/wb_work/hello/test.db";
    clipboardHelper = await nativeBinding.JsClipboardHelper.new(dbPath);
}

async function getClipboardContent() {
    const result = [];
    const clipboardList = await clipboardHelper.getClipboardEntries();
    for (const item of clipboardList.entries) {
        result.push(item.content);
    }
    // 返回倒叙
    return result.reverse();
}

module.exports = {
    initializeClipboardHelper,
    getClipboardContent
};