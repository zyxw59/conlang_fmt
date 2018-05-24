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

### Parameters

Many syntax elements can take optional parameters, which are denoted by a
comma-separated list surrounded by square brackets.

### Blocks

A block is a paragraph-level element, such as a section header, a table, or a
paragraph of text.
All blocks must be separated by blank lines, with no exceptions.

#### Headings and sections

Section headers are denoted by one or more `#` characters, as in Markdown.
Headers are numbered by default.
Parameters are placed immediately after the last `#`.

##### Parameters

- `id`: The ID to assign to the heading.
  Defaults to the text of the heading, with spaces replaced by dashes, and a
  number appended to ensure uniqueness.
- `nonumber`: Do not number this heading.
  If set, the counter for this section level will not increase, and the counter
  for lower levels will not be reset.
  > Note: this parameter should only be used to disable numbering for a single
  > heading.
  > Use CSS to disable numbering for an entire level of headings.
- `notoc`: Do not include this heading in the table of contents.
  > Note: this parameter should only be used to prevent a single heading from
  > appearing in the table of contents.
  > Use parameters on the table of contents itself to hide an entire level of
  > headings.

#### Table of contents

A table of contents can be inserted with a block consisting entirely of the
text `:toc:`, optionally followed by parameters.

##### Parameters

- `maxlevel` (default: 6): The maximum level of section headings to include
  in the table of contents.

#### Bullet lists

Bullet lists are denoted by lines starting with `-` followed by one or more
whitespace characters.
No characters other than `-` are allowed to start list items.
If a list item is too long to fit on a single line, it can be wrapped by
indenting any following lines by two (or more) spaces.
A list item can contain a list, by indenting two (or more) spaces.
> TODO: Where should parameters go

#### Numbered lists

Numbered lists are denoted by lines starting with `!` followed by one ore more
whitespace characters.
They are otherwise identical to bullet lists.
> Note: this notation is subject to change.

#### Tables

Tables are denoted by a block starting with `:table:`, optionally followed by
parameters, and a title for the table.
Tables are automatically numbered.
Rows are denoted by starting a line with `::`.
Cells within a row are delimited by `|`.
Parameters for a row are placed immediately after the `::`.
Parameters for a cell are placed immediately after the `|`.
> TODO: Where should column parameters go

##### Parameters

###### Table

- `id`: The ID to assign to the table.
  If the table has a title, defaults to `table-` plus the title, with spaces
  replaced by dashes, and with a number appended to ensure uniqueness.
  Otherwise, defaults to `table-n`, where _n_ is the number of the table.
- `nonumber`: Do not number this table.
  If set, and the `id` parameter is not set, and the table lacks a title, the
  table's ID will be set to `table-nonumber`, with a number appended to ensure
  uniqueness.
- `class`: The CSS classes to apply to this table.

###### Column

- `header`: If set, the row will be considered a header row, and the cells will
  be `<th scope="row">` elements.
- `class`: The CSS classes to apply to this column.
  Because columns are not logical parent elements of cells, these classes will
  be added to each cell in the column.
  These classes will not be applied to any multi-column cells.

###### Row

- `header`: If set, the row will be considered a header row, and the cells will
  be `<th scope="col">` elements.
- `class`: The CSS classes to apply to this row.

###### Cell

- `cols` (default: 1): The number of columns this cell should span.
- `rows` (default: 1): The number of rows this cell should span.
  In subsequent rows, blank cells should be included where they would be
  covered by an earlier multi-row cell.
  Including any text or parameters in these cells will trigger a warning.
- `class`: The CSS classes to apply to this cell.
  Styling individual cells this way should be done sparingly.

#### Glosses

> TODO: define syntax for glosses

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

> TODO: define syntax for text replacements

#### Cross references

> TODO: define syntax for cross references
