## 开发者无法验证或应用已损坏

> macOS 系统 10.12 版本后对来自非 Mac App Store 中的应用做了限制。

![文件已损坏](./imgs/mac_file_corrupted.png)

- 问题原因: 由于应用没有签名，所以可能会显示开发者无法验证或应用已损坏，需要授予开发者**Apple Developer Program** 会员资格。
- 解决方案1: 点击 `取消` 按钮，然后去 `系统偏好设置` -> `安全性与隐私` 页面，点击 `仍要打开` 按钮，然后在弹出窗口里点击 `打开` 按钮即可。如果你的系统版本较高，可能在 `安全性与隐私` 页面中找不到以上选项，或启动时提示文件损坏。打开终端，并执行下列命令进行授权。

```bash
sudo xattr -d com.apple.quarantine /Applications/Clash\ Verge.app
```
- 解决方案2: 选择允许从任何来源, 进入安全性与隐私，选择允许从任何来源；

```bash
sudo spctl --master-disabl
```

## Apple 无法检查 App 是否包含恶意软件

- 解决方案: 详见[macOS 使用手册](https://support.apple.com/zh-cn/guide/mac-help/mchleab3a043/mac)，并选择对应 mac 版本的文档。

## macOS 菜单栏左上角图标重叠

![菜单栏左上角图标重叠](./imgs/mac_icon_duplicated.png)

- 问题原因: macOS Sonoma 的系统 BUG。
- 解决方案: `系统偏好设置` -> `显示器`，调整一下显示器分辨率，然后再调回去。

## macOS 键入 option(alt) + 字母变成特殊字符，导致录入的快捷方式错误不能正常触发

- 问题原因: macOS 键盘的`option key printing special characters`特性导致，不同的键盘布局有不同的转换关系。
- 解决方案: `系统偏好设置` -> `键盘` -> `输入法` ，添加一种没有配置特殊字符的键盘布局。如何判断键盘布局有没有转换特殊字符？选中某个键盘布局，按下 option(alt)键并观察右侧下半区域**键盘图示上的字母是否发生变化**。挑选一种按下 option（alt）键后字母**变为键盘字母或空白**的键盘布局，如简体中文的`简体笔画`、`五笔型`。
