// clipboardHelper.js
const nativeBinding = require("/Users/zeke/work/workspace/github_work/HostClipboard/core/rs_core/hello.darwin-arm64.node");

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
  // console.log(contentList);

  return contentList;
}

async function searchClipboard(query) {
  console.log("query", query);
  const clipboardList = await clipboardHelper.search(query, 4, -1);
  const contentList = clipboardList.entries.map((item) => item.content);
  console.log("searchClipboard", contentList);
  return contentList;
}

module.exports = {
  initializeClipboardHelper,
  getClipboardContent,
  searchClipboard,
};
