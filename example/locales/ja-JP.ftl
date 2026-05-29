## 1. 基本的なメッセージ
settings = 設定
hello = こんにちは、{ $name }！
item-count = アイテムが { NUMBER($count) } 個あります。

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

# 位置引数は用語の自由変数に順にバインドされる
-greet = { $greeting }、{ $name }！
pos-args-demo = { -greet("やあ", "アリス") }

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

# 用語属性の選択：{ -term.attr } をコンパイル時に解決
-brand-gender =
    { -brand-aurora.gender ->
        [masculine] 様
        [feminine] 様
       *[other] 様
    }
attr-select-demo = 敬称：{ -brand-gender }

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

## 11. カスタム組み込み関数テスト：TEST($value, operator, operand)
test-add-10 = { TEST($value, operator: "+", operand: 10) }
test-sub-5 = { TEST($value, operator: "-", operand: 5) }
test-mul-3 = { TEST($value, operator: "x", operand: 3) }
test-div-2 = { TEST($value, operator: "/", operand: 2) }

## 12. 組み込み関数

### 12a. NUMBER()
dpi-ratio = DPI 比率は { NUMBER($ratio, minimumFractionDigits: 2) } です

### 12b. DATETIME()
today-is = { DATETIME($date, year: 2024, month: 5, day: 17) }
full-date = { DATETIME($date, monthFormat: "long", yearFormat: "numeric", dayFormat: "numeric", year: 2024, month: 5, day: 17) }

## 13. 未サポートの機能

# 13a. 関数呼び出しをセレクターに使用
# your-score =
#     { NUMBER($score, minimumFractionDigits: 1) ->
#         [0.0]   0点でした。
#        *[other] { NUMBER($score, minimumFractionDigits: 1) } 点でした。
#     }

# 13b. NUMBER(&hellip;, type: "ordinal") による序数
# your-rank = { NUMBER($pos, type: "ordinal") ->
#    [1] 1位！
#    [one] { $pos }位
#    [two] { $pos }位
#    [few] { $pos }位
#   *[other] { $pos }位
# }

# 13c. コンテキストに基づく型推論なし
# 変数は自動的に FluentDateTime/FluentNumber として推論されません。
# DATETIME()/NUMBER() 関数を明示的に呼び出す必要があります。
# 以下の $day や $count は単なる文字列パラメータとして扱われます。
# today = 今日は { $day } です
# unread = 未読メッセージが { $count } 件あります。
