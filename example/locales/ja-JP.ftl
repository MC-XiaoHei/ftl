### 包括的な FTL の例 — 日本語
### ftl-codegen がサポートしない機能は §12 でコメントアウト。

## 1. 基本的なメッセージ
settings = 設定
hello = こんにちは、{ $name }！
item-count = アイテムが { $count } 個あります。

## 2. 特殊文字と引用テキスト
brace-demo = 中括弧：{"{"} と {"}"}
preserve-space = {"    "}インデント文
non-breaking = プライバシー{"\u00A0"}ポリシー
tears-of-joy = {"\U01F602"}
tears-of-joy-prefer = 😂
leading-bracket =
    行頭の角括弧：
    {"["}。

## 3. インラインリテラル
answer = 答えは { 42 } です。
inline-str = 値：{"hello"}

## 4. 複数行テキスト
welcome =
    私たちのアプリケーションへようこそ！
    楽しんでいただければ幸いです。
description =
    通常のインデント。
      追加インデント（2スペース分）。
    通常に戻る。
info = 最初の行
    同じ値の続き。
notice =

    上の空行は保持される。

    別の空行。

## 5. メッセージ参照
app-name = マイアプリケーション
about-app = { app-name } について
# 自由変数：`name` を参照すると `$user` が伝播します。
name = { $user }
welcome-user = ようこそ、{ name }！

## 6. 用語
-brand-name = マイアプリケーション
welcome-term = { -brand-name } へようこそ。

## 7. パラメータ化された用語
-brand =
    { $case ->
       *[nominative] マイブランド
        [genitive] マイブランドの
    }
about-brand = { -brand(case: "genitive") } について。
update-brand = { -brand } は更新されました。

## 8. メッセージの属性
login-input = デフォルト値
    .placeholder = メールアドレスを入力
    .aria-label = ログイン入力
    .title = ログインフォーム
product = Zed
save =
    .label = { product } を保存
    .tooltip = 現在の { $target } を保存

# インライン属性参照：{ msg.attr } は属性値に展開される
attr-ref-demo = プレースホルダー：{ login-input.placeholder }

## 9. 用語の属性
-brand-aurora = オーロラ
    .gender = feminine

## 10. 選択式（複数形 / バリアント）
files =
    { $count ->
        [one] 1 ファイル
       *[other] { $count } ファイル
    }
unread-emails =
    { $count ->
        [0] 未読メールはありません
        [one] 1 通の未読メール
       *[other] { $count } 通の未読メール
    }
items =
    { $n ->
        [0] アイテムがありません
        [1] 1 つのアイテム
        [2] 2 つのアイテム
        [42] すべてのアイテム
       *[other] { $n } 個のアイテム
    }
user-greeting =
    { $gender ->
        [male] ご主人様、ようこそ！
        [female] お嬢様、ようこそ！
       *[other] ようこそ！
    }
# ハードコードキーによる順位表現（NUMBER() 未サポート — §12d）
finish-place =
    { $place ->
        [1] 1位になりました！
        [2] 2位になりました！
        [3] 3位になりました！
       *[other] { $place }位になりました！
    }

## 11. コメント（# メッセージレベル、## グループレベル、### ファイルレベル）
settings-page = 設定ページ

## 12. 未サポートの機能（コメントアウト — panic 発生）

# 12a. NUMBER() / 12b. DATETIME() — panic: "Unsupported expression"
# dpi-ratio = DPI 比率は { NUMBER($ratio, minimumFractionDigits: 2) } です
# today-is = 今日は { DATETIME($date) } です
# full-date = { DATETIME($date, month: "long", year: "numeric", day: "numeric") }

# 12c. 関数呼び出しをセレクターに使用 — panic: "Select selector must be a variable"
# your-score =
#     { NUMBER($score, minimumFractionDigits: 1) ->
#         [0.0]   0点でした。
#        *[other] { NUMBER($score, minimumFractionDigits: 1) } 点でした。
#     }

# 12d. NUMBER(…, type: "ordinal") による序数 — panic: "Unsupported expression"
# your-rank = { NUMBER($pos, type: "ordinal") ->
#    [1] 1位！
#    [one] { $pos }位
#    [two] { $pos }位
#    [few] { $pos }位
#   *[other] { $pos }位
# }

# 12e. 用語属性参照 `-term.attr` — .attr は無視される
# -brand-aurora = オーロラ
#     .gender = feminine
# update-status =
#     { -brand-aurora.gender ->
#         [masculine] { -brand-aurora } は更新されました。
#         [feminine] { -brand-aurora } は更新されました。
#        *[other] { -brand-aurora } は更新されました。
#     }

# 12f. 位置引数の用語 — panic: "not supported for '-'"
# -greet = こんにちは、{ $name }！
# say-hi = { -greet("世界") }

# 12g. 非変数セレクター — panic: "Select selector must be a variable"
# always-other = { 42 ->
#     [42] 正確に42
#    *[other] その他
# }

# 12h. 部分フォーマット変数（FluentDateTime / FluentNumber）
#      特別な .ftl 構文は不要；API レベルで値をラップ。
# today = 今日は { $day } です

### ファイル終わり
