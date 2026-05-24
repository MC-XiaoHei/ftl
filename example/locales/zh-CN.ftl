### 完整的 FTL 示例 — 简体中文
### ftl-codegen 不支持的功能在 §12 中注释掉。

## 1. 基本消息
settings = 设置
hello = 你好，{ $name }！
item-count = 您有 { $count } 个项目。

## 2. 特殊字符与引用文本
brace-demo = 花括号：{"{"} 和 {"}"}
preserve-space = {"    "}缩进文本
non-breaking = 隐私{"\u00A0"}政策
tears-of-joy = {"\U01F602"}
tears-of-joy-prefer = 😂
leading-bracket =
    行首方括号：
    {"["}。

## 3. 内联字面量
answer = 答案是 { 42 }。
inline-str = 值：{"hello"}

## 4. 多行文本
welcome =
    欢迎使用我们的应用！
    希望您喜欢。
description =
    普通缩进。
      额外缩进（2 个空格）。
    回到普通。
info = 第一行
    同一个值的内容。
notice =

    上面的空行被保留。

    另一个空行。

## 5. 消息引用
app-name = 我的应用
about-app = 关于 { app-name }
# 自由变量：引用 `name` 会将 `$user` 向上传递。
name = { $user }
welcome-user = 欢迎，{ name }！

## 6. 术语
-brand-name = 我的应用
welcome-term = 欢迎使用 { -brand-name }。

## 7. 带参数的术语
-brand =
    { $case ->
       *[nominative] 我的品牌
        [genitive] 我的品牌的
    }
about-brand = 关于 { -brand(case: "genitive") }。
update-brand = { -brand } 已更新。

## 8. 消息属性
login-input = 默认值
    .placeholder = 输入您的邮箱
    .aria-label = 登录输入框
    .title = 登录表单
product = Zed
save =
    .label = 保存 { product }
    .tooltip = 保存当前的 { $target }

## 9. 术语属性
-brand-aurora = 极光
    .gender = feminine

## 10. 选择表达式（复数 / 变体）
files =
    { $count ->
        [one] 1 个文件
       *[other] { $count } 个文件
    }
unread-emails =
    { $count ->
        [0] 没有未读邮件
        [one] 1 封未读邮件
       *[other] { $count } 封未读邮件
    }
items =
    { $n ->
        [0] 没有项目
        [1] 一个项目
        [2] 两个项目
        [42] 所有项目
       *[other] { $n } 个项目
    }
user-greeting =
    { $gender ->
        [male] 先生，欢迎！
        [female] 女士，欢迎！
       *[other] 欢迎！
    }
# 通过硬编码键实现序数（NUMBER() 不支持 — 见 §12d）
finish-place =
    { $place ->
        [1] 您获得了第一名！
        [2] 您获得了第二名！
        [3] 您获得了第三名！
       *[other] 您获得了第 { $place } 名！
    }

## 11. 注释（# 消息级、## 分组级、### 文件级）
settings-page = 设置页面

## 12. 不支持的功能（已注释 — 取消注释会 panic）

# 12a. NUMBER() / 12b. DATETIME() — panic: "Unsupported expression"
# dpi-ratio = 您的 DPI 比率是 { NUMBER($ratio, minimumFractionDigits: 2) }
# today-is = 今天是 { DATETIME($date) }
# full-date = { DATETIME($date, month: "long", year: "numeric", day: "numeric") }

# 12c. 函数调用作选择器 — panic: "Select selector must be a variable"
# your-score =
#     { NUMBER($score, minimumFractionDigits: 1) ->
#         [0.0]   您得了零分。
#        *[other] 您得了 { NUMBER($score, minimumFractionDigits: 1) } 分。
#     }

# 12d. 通过 NUMBER(…, type: "ordinal") 的序数 — panic: "Unsupported expression"
# your-rank = { NUMBER($pos, type: "ordinal") ->
#    [1] 第一名！
#    [one] 第 { $pos } 名
#    [two] 第 { $pos } 名
#    [few] 第 { $pos } 名
#   *[other] 第 { $pos } 名
# }

# 12e. 术语属性引用 `-term.attr` — 静默丢弃 .attr
# -brand-aurora = 极光
#     .gender = feminine
# update-status =
#     { -brand-aurora.gender ->
#         [masculine] { -brand-aurora } 已更新。
#         [feminine] { -brand-aurora } 已更新。
#        *[other] { -brand-aurora } 已更新。
#     }

# 12f. 位置参数术语 — panic: "not supported for '-'"
# -greet = 你好，{ $name }！
# say-hi = { -greet("世界") }

# 12g. 非变量选择器 — panic: "Select selector must be a variable"
# always-other = { 42 ->
#     [42] 正好四十二
#    *[other] 其他
# }

# 12h. 部分格式化变量（FluentDateTime / FluentNumber）
#      无特殊 .ftl 语法；开发者在 API 层包装值。
# today = 今天是 { $day }

### 文件结束
