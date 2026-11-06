# Gen MOD 深色界面与品牌设计

## 已确认的设计

采用讨论中选定的「近黑 · 浅金」和「小按钮 · 其余默认」方案，Logo 采用 A 方向的模块 G。本文是本轮 UI 调整的实施依据。

- 允许调整现有页面的排版、入口呈现和文案，不增加功能。
- 保留项目、会话、Skills、设置、源码、版本与安装等现有功能及数据流。
- 项目页只展示项目及其现有操作，不加入需求输入框、虚构导航、搜索、筛选或额外状态面板。
- 本轮不调整接口、模型行为、存储、权限和业务校验，不新增依赖。
- 产品面向 PC 桌面，不提供移动端适配；布局需要适应桌面窗口缩放和现有最小窗口尺寸。

## 配色

以近黑和微暖的中性灰建立层次，浅金只用于主要操作、焦点与少量选中标识。不使用青色、霓虹光效、全屏金色染色、金属质感或装饰性网格。

| 用途 | 色值 |
| --- | --- |
| 主画布 | `#131212` |
| 侧栏 | `#181615` |
| 卡片、弹出面板 | `#1E1C1B` |
| 次级表面、悬停 | `#282421` |
| 边框 | `#302C29` |
| 输入框边框 | `#494039` |
| 主文字 | `#F1EFEC` |
| 次级文字 | `#A59E97` |
| 主操作、焦点、品牌图形 | `#F2CA8F` |
| 浅金按钮内文字 | `#231B11` |
| 侧栏选中背景 | `#25201C` |
| 错误提示 | `#FF8D91` |

参考 [Figma 邀请活动面板](https://www.figma.com/design/Bhrr8t5g0giYtbAxwpzLOU/?node-id=2000-90404) 中的 `#F2CA8F` 和 4% 白色叠层关系。实现使用近似叠层结果的实体面板色，避免嵌套透明度造成色差；不照搬活动页插画、票券或装饰线条。

成功、错误、差异增删等已有语义颜色继续保留，不将所有状态统一涂金。源码高亮与 Markdown 继续使用已有暗色支持。

## 尺寸与排版

- 页面正文保持 14px；辅助信息 12px；页面标题 24px；项目名称 16px；常规图标 16px。
- 字体沿用本机现有字体，不下载字体；正文使用 `Segoe UI Variable`、`Microsoft YaHei UI` 等现有回退。
- 常规操作按钮默认使用 `sm`：高 32px、文字 13px、水平内边距 12px、圆角约 5px。默认尺寸按钮仍保留为显式变体。
- 常规图标按钮高宽 32px；已有附件移除、菜单等专用更小按钮保留尺寸。按钮内部常规图标不随文字缩小。
- 导航行、输入框、选择器、正文和卡片不跟随按钮一起缩小。已有业务组件用于长内容的 Button 保留正文的 14px 排版。
- 页面内容宽度上限 1024px；常规页内边距 32px，较窄窗口 24px。侧栏宽度 224px。
- 项目卡片最多两列，空间不足时单列。保留完整点击区域、标题截断、描述两行省略及真实版本数量。
- 卡片不使用渐变、光晕、彩色大图标底座或悬停装饰条。分隔线、边框和轻微底色变化负责层次。

## 页面调整

### 项目页

页头使用「MOD 项目」，说明为「从一个想法开始，保留每次修改与交付。」。右侧保留「已归档项目」和「新建 MOD」。主区保留项目卡片及「移除」，移除上轮未选定的大型宣传标题与重复项目库计数标题。

空状态保留已有创建入口。卡片展示项目名、需求摘要、游戏名称与交付版本数量，不添加新的项目属性。

### 侧栏

内容区域不显示 Logo 和 Gen MOD 字标，避免与系统标题栏重复。折叠按钮并入「全部项目」行的右侧，不保留空品牌区。保留全部项目、项目展开收起、会话、操作菜单、Skills、设置和折叠导航；选中背景保持低对比，不整行高亮金色文字。

系统标题栏保留原生布局、图标、标题及窗口按钮，仅将窗口 `theme` 设置为 `Dark`，与内容区的近黑色协调；不新增自绘标题栏、窗口权限或控制逻辑。原生标题栏的具体深色由操作系统绘制，不承诺与内容画布逐像素同色。该配置用于窗口创建，实际效果在窗口重新创建后确认。配置依据见 [Tauri WindowConfig.theme](https://v2.tauri.app/reference/config/#theme-1)。

### 聊天、Skills、设置及弹窗

聊天保留独立页面及现有输入区，输入框不进入项目页。取消空状态装饰网格和金色辅助文字；保留发送、停止、附件、模型和思考强度的原有逻辑。

Skills 与设置使用同样的标题、间距、表面和边框，移除多余英文眉题；标题分别为「Skills」和「设置」。表单标签、输入框及说明保持默认字号。新建、移除、归档等现有弹窗沿用原交互，仅继承统一主题和小尺寸操作按钮。

思考强度弹层的标题、填充与滑块标识复用浅金主题色，保留原档位和滑动操作，不使用独立蓝紫色、星点渐变或宽度过渡动画。版本管理及模型工具栏的常规文字操作按钮统一采用 Small，专用图标按钮保留原用途和尺寸。

模型设置采用桌面固定操作区：标题和页签保持可见，中间列表独立滚动，底部的保存状态、保存错误、「放弃修改」「保存配置」始终可见。服务编辑弹窗仍只添加或更新到列表，由用户统一保存；文案明确提示尚需保存。加载和保存期间维持原有禁用保护。

面向用户的基础操作统一使用「游戏」「模型服务」「能力配置」等具体名称，保留必要的 API Key、Base URL、ID 和 JSON 标识；不更改接口字段及内部代码标识符。

## Logo

使用 A 方向：两个几何模块形成清晰的 G，保留斜向切口与内部负空间。图形为纯浅金 `#F2CA8F`，不加入文字、星芒、渐变、金属纹理或立体厚度。

- `public/gen-mod-logo.svg`：透明底、可缩放的界面图形源文件。
- `public/gen-mod-app-icon.svg`：近黑圆角底座的应用图标源文件。
- `public/gen-mod-icon.png`：应用图标的 PNG 导出，用于现有 favicon 等入口。
- `src-tauri/icons/`：从同一应用图标源文件生成，更新仓库原有 PNG、ICO 和 ICNS，不添加平台功能。
- 透明底 SVG 保留为品牌源文件，内容侧栏不显示 Logo；不将生成草案中的字标、标签或其他候选图形带入产品。

先使用内置图像工具生成选定方向，再按其轮廓整理为可维护的纯色 SVG。最终图形以 SVG 为准，PNG 与系统图标统一从该源文件导出，避免生成图中的纹理和透明边缘噪点。

### 图像生成记录

使用内置 `image_gen`，以用户选定的三方案对比图为参考，要求透明背景。生成结果作为造型参考，产品使用清理后的 SVG。提交的提示词如下：

```text
Use case: logo-brand / precise-object-edit. The supplied comparison board is the reference. Extract and faithfully reproduce ONLY the large LEFTMOST mark labelled A: the recognisable uppercase G assembled from two angular geometric modular pieces, diagonal gap across the left side and an inward horizontal G bar on the right. Preserve the selected A design's exact recognizable proportions, silhouette, diagonal cuts and lightly rounded outer corners. Produce ONE standalone centered app logo mark with no letters other than the shape of the G itself. No wordmark, no A label, no comparison board, no small logos, no other symbols. Completely flat uniform pale gold #F2CA8F, absolutely no gradient, lighting, texture, shadow, metallic effect, bevel or 3D. Truly transparent background, keep transparency in the negative space of the G and between its pieces. A square image with the mark occupying about 78% of width and height, even clear padding on all sides. Crisp sharp vector-like edges, professional production-ready logo mark intended to remain legible at 24px. No colored backdrop and no dark app tile; just the pale-gold A modular G mark on transparent canvas.
```

### 图标导出

图标导出使用已有 Tauri CLI（不新增工具依赖）：

```powershell
node node_modules/@tauri-apps/cli/tauri.js icon public/gen-mod-app-icon.svg --output work/brand-icons
```

仅将与 `src-tauri/icons/` 原有文件同名的输出复制回图标目录；将生成的 `icon.png` 同步到 `public/gen-mod-icon.png`。应用图标的操作系统缓存及安装包展示需在后续实际启动或构建时确认。

## 验证边界

本轮按项目约定执行限定文件 Lint、Vue 语法编译、差异和资源引用检查，以及 `codegraph sync`、`codegraph status`。不运行测试、不启动应用、不执行全仓类型检查或构建；静态检查不代表视觉验收或系统图标缓存已更新。

## 实施完成后的人工确认

- 项目页、空状态、聊天、Skills、设置与现有弹窗的整体协调性。
- Small 按钮与默认字号、输入框、导航的搭配。
- 内容侧栏无重复品牌区；favicon 以及重新构建后的桌面和安装包图标。
- 窄窗口下的标题、操作区换行和卡片布局。
