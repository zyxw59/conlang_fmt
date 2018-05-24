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

The `class` parameter can be abbreviated by leaving a space-separated list of
classes as the final parameter.
This does not work when only one class is included and that class conflicts
with a named parameter of that element.
For example, a heading with the parameter string `[notoc]` would be parsed as
having the `notoc` parameter, rather than a class of `notoc`.

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
- `class`: The CSS classes to apply to this heading.
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

- `class`: The CSS classes to apply to the table of contents.
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
  These classes will be applied to the containing `<tr>` element, and _do_
  apply to multi-row cells.

###### Cell

- `cols` (default: 1): The number of columns this cell should span.
- `rows` (default: 1): The number of rows this cell should span.
  In subsequent rows, blank cells should be included where they would be
  covered by an earlier multi-row cell.
  Including any text or parameters in these cells will trigger a warning.
- `class`: The CSS classes to apply to this cell.
  Styling individual cells this way should be done sparingly.

#### Glosses

Glosses are denoted by a block starting with `:gloss:`, optionally followed by
parameters, and a title for the gloss.
Glosses are automatically numbered.
New lines of the gloss are denoted by starting a line with `::`.
A gloss line consists of a series of space-separated words.
If a logical word contains a space, it can be surrounded by curly braces.
In the output, a space will be inserted between two gloss elements unless the
first one ends with a `-` character, or the second one begins with a `-`
character.
Parameters for a line are placed immediately after the `::`.

##### Parameters

###### Gloss

- `id`: The ID to assign to the gloss.
  If the gloss has a title, defaults to `gloss-` plus the title, with spaces
  replaced by dashes, and with a number appended to ensure uniqueness.
  Otherwise, defaults to `gloss-n`, where _n_ is the number of the gloss.
- `nonumber`: Do not number this gloss.
  If set, and the `id` parameter is not set, and the gloss lacks a title, the
  gloss's ID will be set to `gloss-nonumber`, with a number appended to ensure
  uniqueness.
- `class`: The CSS classes to apply to this gloss.

###### Line

- `nosplit`: Do not split this line up into words.
  The line will not be considered a part of the gloss.
  `nosplit` lines cannot come in between regular gloss lines -- they must all
  come at the beginning and/or the end of the gloss.
- `class`: The CSS classes to apply to this line.

### Inline elements

Inline elements can be included inline in text.

#### Formatting

- Emphasis (usually displayed as italics) is indicated by surrounding the text
  with `*`.
- Strong emphasis (usually displayed as bold) is indicated by surrounding the
  text with `**`.
- Italics (formatting only, without semantics) is indicated by surrounding the
  text with `_` (a single underscore).
- Bold (formatting only, without semantics) is indicated by surrounding the
  text with `__` (two underscores).
- Small caps is indicated by surrounding the text with `^^`.
- A generic `<span>` element is indicated by surrounding the text with `` ` ``.

In each of these cases, parameters come directly after the closing delimiter.

##### Parameters

- `class`: The CSS classes to apply to this span.
  In the case of a generic span, this defaults to `conlang`.
  Otherwise, defaults to none.

#### Text replacements

Text replacements can be defined in a block starting with `:replace:`, followed
by a list of replacements.
Each replacement in the list should consist of the identifier for the
replacement, surrounded by `:`, followed by the replacement text, which may
itself contain replacements (or any other inline formatting).
In the text, a replacement is denoted by the replacement's identifier,
surrounded by `:`.
Text replacements do not take any parameters.

#### Cross references

Cross references are denoted with `:ref:`.
The parameters come immediately afterwards.

##### Parameters

- `ref`: The ID to reference in the document.
  The text for the reference will automatically be set based on the type of
  element it refers to: "section", "table", or "gloss"; followed by the number
  of that element.
  If the reference points to an element with the `nonumber` parameter, then a
  warning will be raised, and the text will simply be the type of the element.
- `class`: The CSS classes to apply to this reference.
