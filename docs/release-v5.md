# GridCraft 0.19.1 v5：粘贴保形状 + 刷新外部链接

这一版接着 v4（中文字体与粗体），补上两件还没做的事：粘贴时保住表格的形状，
以及刷新指向别的 Excel 文件的链接。

## 这一版改了什么

- **粘贴保留合并单元格**：剪贴板 HTML 里的 `colspan` / `rowspan` 就是合并区，现在在目标位置做出
  同样的合并区，内容放在左上角，其余格子留空。合并区会按粘贴写入的范围和标记留空的格子收紧，
  不会把源里没有的内容吞进去
- **粘贴保留边框**：读并应用标记里的 `border`、`border-top` 等——标了哪条边就换哪条边，
  没标的边保留目标格子自己的
- **粘贴保留格内粗体**：单元格里有粗体片段（`<b>`，或 `font-weight:bold` 的 span）时整格加粗。
  单元格模型里没有片段这一层，这是近似做法，代码里注明了
- **刷新其他文件的链接**：Data → Links → Refresh External Links。
  工作簿链接别的 Excel 文件时，GridCraft 会显示 Excel 最后存下的值，但不会自己去更新；
  现在可以刷新：指定链接指向的文件，程序打开它，把引用它的单元格更新成那个文件里的当前值。
  公式保留，链接仍是链接，Excel 打开照样解析。文件打不开时不动任何值并说明原因。
  Data → Links → External Links… 可以先看这个工作簿有哪些链接

v4 已经有的（中文字体名认得出、粗体真的粗、字体下拉列出本机真实装的字体）这一版继续有。

## 使用说明（macOS 未签名版）

1. 下载 release 里的 `gridcraft-0.19.1-v5-macos-aarch64.dmg`（Apple Silicon：M1/M2/M3/M4/M5）
2. 终端里解除隔离标记（不需要密码）：
   `xattr -dr com.apple.quarantine ~/Downloads/gridcraft-0.19.1-v5-macos-aarch64.dmg`
3. 双击 dmg，把 GridCraft 拖进 Applications
4. 第一次打开：在 GridCraft 上右键 → 打开 → 再点「打开」

别点应用自己的「更新」提示：一更新就换回官方版，本页这些改动就没了。
