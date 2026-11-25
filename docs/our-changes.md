# 我们对 GridCraft 的改动（逐版本）

本文只记录我们在上游 GridCraft（github.com/storytold/gridcraft，v0.19.1）基础上每一版做了什么。
每一版在 GitHub release 上有对应的下载页。

## 改动

### v1 —— 批注的时间与人名（`v0.19.1-comment-dates`）

- 新建的批注和每一条回复都记 ISO 8601 时间戳，写进 XLSX 的 `threadedComments` 的 `dT`，
  所以 Excel 打开也能看到时间；回读时按本地时区显示成 `11/20/26 1:09 PM`
- Review → Show Comments 面板与单元格悬浮框都显示「作者 + 时间 + 正文」，每条回复各占一块
- 回复框改为多行：Enter 换行，⌘↩（或 Send）发送
- File → Options → Comments 新增「User name on comments and replies」，控制批注署名

### v2 —— Excel 式批注卡片（`v0.19.1-comment-card`）

v1 的回复框在右侧面板里，得先找到面板才能回复；这一版把它搬到单元格旁边：

- 有批注的单元格右上角出现**评论三角标**（Excel 的红三角），点它就在格子旁边打开**批注卡片**
- 卡片里逐条列出：作者名 + 写入时间 + 正文；底部直接回复，Enter 换行、⌘↩ 发送
- 右键单元格 → New Comment（已有批注时显示 Edit Comment）、ribbon 的 New Comment、⌘F2，
  都打开这张卡片（v1 的旧对话框只保留给"便签/Note"）
- 卡片按 Esc 或点击别处关闭

### v3 —— 格式、跨表引用、中文字库（`v0.19.1-v3`）

- **粘贴保留格式**：以前从别的 Excel 复制过来只搬文字和数字，格式全丢。现在读剪贴板 HTML 里
  `<style>` 块定义的类（Excel 把格式写在 `class="xl65"` 里，不是内联样式）以及单元格自己的
  `style` 属性，保留粗体、斜体、下划线、字体颜色、字体名、字号、单元格底色、对齐和数字格式
- **跨表引用不再被排序弄乱**：排序时 `=Rates!A2` 这种指向别的表的引用会跟着记录走（上游 #3781），
  每条记录还是配它原来的数
- **复制工作表后自引用跟着副本**：`=Source!A1` 在 `Source` 的副本上改成 `='Source (2)'!A1`
  （上游 #3763），副本读自己的数据
- **内嵌中文字库**：构建时嵌入官方 release 同款的 craft-fonts 字面（含简体中文字面），
  中文不再靠机器上碰巧装了什么字体
- macOS 构建只出 Apple Silicon 一个包（更快、体积更小）

## v4 —— 中文字体与粗体（`v0.19.1-v4`）

- **中文字体名认得出了**：工作簿里写「宋体」，这台机器上叫「宋体-简 / Songti SC」；以前名字对不上，
  单元格就退回通用字体。现在「宋体/黑体/楷体/仿宋/微软雅黑/等线/苹方/华文系列」都能找到本机对应的
  家族；Windows 独有的字体（微软雅黑等）用系统自带的中文家族顶上
- **粗体真的粗了**：中文（以及阿拉伯文、希伯来文、泰文、韩文）的粗体面现在按字重单独加载，
  不再拿常规字重的字来画——之前粗体单元格里的中文看着是细的
- **字体下拉列出本机真实装的字体**：以前只有 17 个写死的英文字体名，macOS 装的中文字体一个都选不到

## 使用说明（macOS 未签名版）

1. 下载 release 里的 `gridcraft-*-macos-aarch64.dmg`（Apple Silicon；Intel 机器需要 universal 包）
2. 终端里解除 Gatekeeper 的拦截（不需要密码）：
   `xattr -dr com.apple.quarantine ~/Downloads/<文件名>`
3. 双击 dmg，把 GridCraft 拖进 Applications
4. 第一次打开：在 GridCraft 上右键 → 打开 → 再点「打开」

别点应用自己的"更新"提示：一更新就换回官方版，本页这些改动就没了。
