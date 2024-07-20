// clipboardHelper.js
const nativeBinding = require("/Users/zeke/work/workspace/py_work/HostClipboard/core/rs_core/hello.darwin-x64.node");

let clipboardHelper;

async function initializeClipboardHelper() {
    // Note: The new() method doesn't take a dbPath parameter anymore
    // It optionally takes logLevel and sqlLevel
    clipboardHelper = await nativeBinding.JsClipboardHelper.new(4, 2);
}

async function getClipboardContent() {
    // Assuming we want to get all entries (passing a large number)
    const clipboardList = await clipboardHelper.getClipboardEntries(1000);

    const sortedEntries = clipboardList.entries.sort(
        (a, b) => b.timestamp - a.timestamp,
    );

    // Extract content
    const contentList = sortedEntries.map((item) => item.content);

    return contentList;
}

async function searchClipboard(query) {
    console.log("query", query);
    // Using searchClipboardEntries instead of search
    // Assuming we want to search all types (undefined for typeInt)
    const clipboardList = await clipboardHelper.searchClipboardEntries(
        query,
        4,
    );

    const contentList = clipboardList.entries.map((item) => item.content);
    // console.log("searchClipboard", contentList);

    return contentList;
}

async function refreshClipboard() {
    await clipboardHelper.refreshClipboard();
}

module.exports = {
    initializeClipboardHelper,
    getClipboardContent,
    searchClipboard,
    refreshClipboard,
};
