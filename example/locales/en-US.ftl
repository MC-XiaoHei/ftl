### Comprehensive FTL example — en-US
### Unsupported features are commented out in §12.

## 1. Basic Messages
settings = Settings
hello = Hello, { $name }!
item-count = You have { $count } items.

## 2. Special Characters & Quoted Text
brace-demo = Brace: {"{"} and {"}"}
preserve-space = {"    "}Indented text
non-breaking = Privacy{"\u00A0"}Policy
tears-of-joy = {"\U01F602"}
tears-of-joy-prefer = 😂
leading-bracket =
    Line starts with a bracket:
    {"["}.

## 3. Inline Literals
answer = The answer is { 42 }.
inline-str = Value: {"hello"}

## 4. Multiline Text
welcome =
    Welcome to our application!
    We hope you enjoy your stay.
description =
    Normal indent.
      Extra indent (2 extra spaces).
    Back to normal.
info = First line
    and this is the same value.
notice =

    Blank line above is preserved.

    Another blank line above.

## 5. Message References
app-name = My Application
about-app = About { app-name }
# Free variable: referencing `name` propagates `$user` upward.
name = { $user }
welcome-user = Welcome, { name }!

## 6. Terms
-brand-name = My Application
welcome-term = Welcome to { -brand-name }.

## 7. Parameterized Terms
-brand =
    { $case ->
       *[nominative] MyBrand
        [genitive] MyBrand's
    }
about-brand = About { -brand(case: "genitive") }.
update-brand = { -brand } has been updated.

## 8. Attributes on Messages
login-input = Default value
    .placeholder = Enter your email
    .aria-label = Login input
    .title = Login form
product = Zed
save =
    .label = Save { product }
    .tooltip = Save the current { $target }

# Inline attribute reference: { msg.attr } expands to the attribute value
attr-ref-demo = Placeholder: { login-input.placeholder }

## 9. Attributes on Terms
-brand-aurora = Aurora
    .gender = feminine

# Term attribute select: resolves { -term.attr } at compile time
-brand-gender =
    { -brand-aurora.gender ->
        [masculine] Mr.
        [feminine] Ms.
       *[other] Mx.
    }
attr-select-demo = Title: { -brand-gender }

## 10. Select Expressions (plurals / variants)
files =
    { $count ->
        [one] 1 file
       *[other] { $count } files
    }
unread-emails =
    { $count ->
        [0] No unread emails
        [one] 1 unread email
       *[other] { $count } unread emails
    }
items =
    { $n ->
        [0] No items
        [1] One item
        [2] Two items
        [42] All items
       *[other] { $n } items
    }
user-greeting =
    { $gender ->
        [male] Welcome, sir!
        [female] Welcome, ma'am!
       *[other] Welcome!
    }
# Ordinal via hard-coded keys (NUMBER() unsupported — see §12d)
finish-place =
    { $place ->
        [1] You finished first!
        [2] You finished second!
        [3] You finished third!
       *[other] You finished { $place }th!
    }

## 11. Comments (# message-level, ## group-level, ### file-level)
settings-page = Settings Page

## 12. Unsupported Features (commented out — would panic)

# 12a. NUMBER() / 12b. DATETIME() — panic: "Unsupported expression"
# dpi-ratio = Your DPI ratio is { NUMBER($ratio, minimumFractionDigits: 2) }
# today-is = Today is { DATETIME($date) }
# full-date = { DATETIME($date, month: "long", year: "numeric", day: "numeric") }

# 12c. Function call as selector — panic: "Select selector must be a variable"
# your-score =
#     { NUMBER($score, minimumFractionDigits: 1) ->
#         [0.0]   You scored zero points.
#        *[other] You scored { NUMBER($score, minimumFractionDigits: 1) } points.
#     }

# 12d. Ordinal via NUMBER(…, type: "ordinal") — panic: "Unsupported expression"
# your-rank = { NUMBER($pos, type: "ordinal") ->
#    [1] You finished first!
#    [one] You finished { $pos }st
#    [two] You finished { $pos }nd
#    [few] You finished { $pos }rd
#   *[other] You finished { $pos }th
# }

# 12e. Positional term arguments — panic: "not supported for '-'"
# -greet = Hello, { $name }!
# say-hi = { -greet("World") }

# 12h. Partially-formatted variables (FluentDateTime / FluentNumber)
#      No special .ftl syntax; the developer wraps the value at the API level.
# today = Today is { $day }

### EOF
