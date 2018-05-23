# conlang_fmt

A program for formatting constructed language documentation.
This program is bespoke for that purpose; attempting to use it for anything
else should be done with the knowledge that it will likely be difficult at
best, and impossible at worst.
Over time, as more features are added, that may change, but for now, don't
expect it to do everything.

## Syntax

> Since no code has been written yet, this section will define the desired
> features and syntax for the program.

### Blocks

A block is a paragraph-level element, such as a section header, a table, or a
paragraph of text.
All blocks must be separated by blank lines, with no exceptions.

#### Headings and sections

Section headers are denoted by one or more `#` characters, as in Markdown.
Headers are numbered by default.
[TODO: customization for formatting, presence of section numbers]

#### Table of contents

A table of contents can be inserted with a block consisting entirely of the
text `:toc:`.
[TODO: parameters for table of contents]

#### Bullet lists

Bullet lists are denoted by lines starting with `-` followed by one or more
whitespace characters.
No characters other than `-` are allowed to start list items.
If a list item is too long to fit on a single line, it can be wrapped by
indenting any following lines by two (or more) spaces.
A list item can contain a list, by indenting two (or more) spaces.

#### Numbered lists

Numbered lists are denoted by lines starting with `!` [subject to change]
followed by one ore more whitespace characters.
They are otherwise identical to bullet lists.

#### Tables

Tables are denoted by a block starting with `:table:`.
Rows are denoted by starting a line with `::`.
Cells within a row are delimited by `|`.
[TODO: parameters for tables and cells]

#### Glosses

[TODO: define syntax for glosses]

### Inline elements

Inline elements can be included inline in text.

#### Formatting

Emphasis (usually displayed as italics) is indicated by surrounding the text
with `*`.
Strong emphasis (usually displayed as bold) is indicated by surrounding the
text with `**`.
Italics (formatting only, without semantics) is indicated by surrounding the
text with `_` (a single underscore).
Bold (formatting only, without semantics) is indicated by surrounding the
text with `__` (two underscores).
Small caps is indicated by surrounding the text with `^^`.

#### Custom `span` classes

To generate a `<span>` element with custom CSS classes, surround the text with
`` ` ``, followed by `[` + _list of classes_ + `]`.
When used without a trailing `[…]`, the span will have the class `conlang`.

##### Example

The following snippet:
```
Normal text, `conlang text` `custom classes`[my-class another-class].
```
will produce the HTML output:
```html
Normal text, <span class="conlang">conlang text</span> <span class="my-class another-class">custom classes</span>.
```

#### Text replacements

[TODO: define syntax for text replacements]

#### Cross references

[TODO: define syntax for cross references]
