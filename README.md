[mdbook](https://github.com/rust-lang/mdBook) will render extra space of
continuous lines with CJK characters.

```
.....中文结尾
中文顶格...

will result in

.....中文结尾 中文顶格...
             `- note the space here
```

This preprocessor will fix that.

# Usage

1. Download the binary from the release page and put it in your `PATH`.
2. Add the following config to your `book.toml`
    ```
    [preprocessor.fix-cjk-spacing]
    command = "mdbook-fix-cjk-spacing"
    ```
3. Done

# How does it work?

This preprocessor will work on AST of the markdown file:

1. It will use [pulldown-cmark](https://github.com/raphlinus/pulldown-cmark) to parse the markdown file.
2. When encounter a `SoftBreak` token, it will search before and after for a `Text` token.
3. The `SoftBreak` is omitted when the previous text ends with CJK and next text starts with CJK character.

The binary has a "raw" mode for showing the processed output:

```sh
cat markdown.md | md-fix-cjk-spacing raw
```

# The problem

In markdown, if we write several lines continuously, it will be parsed as a
whole block:

```
line 1
line 2
line 3

// will be parsed as

<p>line 1
line 2
line 3</p>
```

That means line breaks are kept and all the three lines are treated as a whole
paragraph.

However, the browser will convert the line break in a `<p>` into a single
space, so when we see the previous content in a browser, it will look like:

```
line 1 line 2 line 3
```

That is OK except when we use Chinese. There is no concept of space in
Chinese, so when we write:

```
中文第一行
中文接上行

// will show as

中文第一行 中文接上行
//        `- not the space here
```

It is really frustrating! So there are two major solutions:

1. Fixing the markdown parsing code to treat it correctly.
2. Write the whole paragraph in a long line.

The first option is actually not so practical. This 'bug' exist for so long
and still not fixed. The second will be so boring and un-friendly.

So here comes our solution with `mdbook`: Write a preprocessor to merge
Chinese lines automatically before parsing!

# The use case

Only the following situation are dealt with:

```
...<chinese character>[should contains no spaces]
[zero or more spaces|tab]<chinese character>

.....中文结尾
中文顶格...

// are modified to
.....中文结尾中文顶格...
//           `- note no space here
```

Note that the content in code block will *not* be changed.
