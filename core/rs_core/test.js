const nativeBinding = require("./hello.darwin-arm64.node");

async function main() {
    const res = [];
    const dbPath = "/Users/zeke/work/workspace/wb_work/hello/test.db";
    const helper = await nativeBinding.JsClipboardHelper.new(dbPath);
    const clipboardList = await helper.getClipboardEntries();
    console.log(clipboardList.entries);
    for (const entry of clipboardList.entries) {
        console.log(entry.content);
        res.push(entry);
    }
}
main().catch(console.error);
