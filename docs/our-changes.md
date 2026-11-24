# 我们对 GridCraft 的改动与使用说明

本文记录我们在上游 GridCraft（github.com/storytold/gridcraft，v0.19.1）基础上做的每一轮改动，
以及这版 macOS 版的用法。源码在本 fork：`github.com/guzyang/gridcraft-mac-build`。

每一版都在 GitHub release 上有对应的下载页（Assets 里是未签名的 .dmg）。

## 安装（macOS，未签名版）

1. 下载 `gridcraft-*-macos-aarch64.dmg`（Apple Silicon；Intel 机器需要 universal 包）
2. 终端里解除 Gatekeeper 的拦截（不需要密码）：

   ```
   xattr -dr com.apple.quarantine ~/Downloads/gridcraft-0.19.1-v3-macos-aarch64.dmg
   ```

3. 双击 dmg，把 GridCraft 拖进 Applications
4. 第一次打开：在 GridCraft 上右键 → 打开 → 再点「打开」
   （提示"无法验证开发者"就去 系统设置 → 隐私与安全性 → 仍要打开）

**注意**：别点应用自己的"更新"提示。更新会把我们改过的版本换回官方版，这些改动就没了。

## 各版本改动

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

## 还没做的

- **外部工作簿链接**：指向另一个 .xlsx 文件的单元格链接还不支持（上游 #3644）。本轮只做到
  "打开时能显示 Excel 缓存的值、不报错"，还不能自动刷新
- **粘贴时的合并单元格与边框**：目前会丢（下一轮做）
- **一格内多种颜色的富文本**：不支持（上游 #774）
- **中文粗体**：见下一节
- **字体下拉里选不到系统中文字体**：目前只列 17 个写死的英文字体名（下一轮做）

## 中文粗体的原因与下一步（计划）

原因：GridCraft 的字体绑定是**按名字精确匹配**的。macOS 上宋体叫「宋体-简 / Songti SC」，
工作簿里写的是「宋体 / SimSun」，名字对不上 → 退回通用 CELL_BOLD 角色 → 中文由脚本回退字体
（PingFang / Hiragino / Noto CJK 的**常规字重**）绘制，于是中文看起来没有加粗。

计划三步：

1. **别名映射**：把工作簿常见中文字体名映射到系统里的实际字体名（宋体→宋体-简/Songti SC、
   黑体→黑体-简/Heiti SC、楷体→楷体-简/Kaiti SC、微软雅黑→Microsoft YaHei …），
   映射后走已有的 `pick_face` 逻辑，直接选中系统里**真正的粗体字面**
2. **脚本回退按字重选面**：回退链目前与粗细无关。改为按字重挑面（用已有的字体名表解析找
   Bold/Semibold/Medium 字面），中文、阿拉伯文、希伯来文、泰文、天城文一并受益
3. 找不到任何粗体字面时，最后才考虑"合成加粗"（同一字形偏移画两次），这层要改绘制路径，放最后

## 验证方式

- 引擎与公式：`cargo test -p gridcraft-engine -p gridcraft-formula`（八戒上跑）
- 界面：`cargo test -p gridcraft-ui-egui`
- 每一轮改动都有对应的回归测试（排序带记录、复制表带自引用、粘贴保格式、批注卡片渲染）
