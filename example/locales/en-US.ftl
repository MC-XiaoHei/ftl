settings = Settings

hello = Hello, { $name }!

files =
    { $count ->
        [one] 1 file
       *[other] { $count } files
    }
