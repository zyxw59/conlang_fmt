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
If a parameter takes an argument, it is denoted by an equals sign followed by
the value of the argument.

#### Common Parameters

- `class`: A list of CSS classes to apply to the element.
  The `class` parameter can be abbreviated by leaving a space-separated list of
  classes as the final parameter.
  This does not work when only one class is included and that class conflicts
  with a named parameter of that element.
  For example, a heading with the parameter string `[notoc]` would be parsed as
  having the `notoc` parameter, rather than a class of `notoc`.
- `id`: The ID for the element.
  This parameter is only allowed on block-level directives.
  By default, the ID is composed of three elements, joined by `-`:
  - The type of the block (e.g. `section`, `table`, `gloss`)
  - The title of the block, if it has one, with spaces replaced by `-`.
    Otherwise, if the block is numbered, the number of the block.
    If the block lacks both a title and a number, then simply `nonumber`.
  - A numeric suffix to ensure uniqueness.
    If the ID is already unique, this element will be ommitted, including the
    preceding `-`.

### Directives

A directive is indicated by text surrounded by colons, with the exception of
headings (indicated by a series of `#` characters), and some formatting
commands (indicated by surrounding text with various delimiters).
The parameters for a directive go directly after the second colon.

### Blocks

A block is a paragraph-level element, such as a section header, a table, or a
paragraph of text.
All blocks must be separated by blank lines, with no exceptions.

#### Headings and sections

Section headers are denoted by one or more `#` characters, as in Markdown.
Headers are numbered by default.
Parameters are placed immediately after the last `#`.

##### Parameters

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

#### Table of contents (`:toc:`)

The directive can optionally be followed by a title for the table of contents,
which defaults to "Table of contents".

##### Parameters

- `maxlevel` (default: 6): The maximum level of section headings to include
  in the table of contents.

#### Lists (`:list:`)

Each element of a list is denoted by a line starting with `::`.
A list item can contain a list, by indenting the entire sub-list by two (or
more) spaces.
By default, lists are unordered lists (bullet points).

##### Parameters

- `ordered`: Make the list an ordered list.

#### Tables (`:table:`)

The directive can optionally be followed by a title for the table.
Tables are automatically numbered.
Rows are denoted by starting a line with `::`.
Cells within a row are indicated by a preceding `|`.
Parameters for columns are placed in a row not starting with `::`.
Parameters for a row are placed immediately after the `::`.
Parameters for a cell are placed immediately after the `|`.

##### Parameters

###### Table

- `nonumber`: Do not number this table.

###### Column

- `header`: If set, the row will be considered a header row, and the cells will
  be `<th scope="row">` elements.
> Note about `class`: Because columns are not logical parent elements of cells,
> classes will be added to each cell in the column.
> These classes will not be applied to any multi-column cells.

###### Row

- `header`: If set, the row will be considered a header row, and the cells will
  be `<th scope="col">` elements.
> Note about `class`: Classes are applied to the containing `<tr>`
> element, and _do_ apply to multi-row cells starting in this row.

###### Cell

- `cols` (default: 1): The number of columns this cell should span.
- `rows` (default: 1): The number of rows this cell should span.
  In subsequent rows, blank cells should be included where they would be
  covered by an earlier multi-row cell.
  Including any text or parameters in these cells will trigger a warning.

#### Glosses (`:gloss:`)

The directive can optionally be followed by a title for the gloss.
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

- `nonumber`: Do not number this gloss.
  If set, and the `id` parameter is not set, and the gloss lacks a title, the
  gloss's ID will be set to `gloss-nonumber`, with a number appended to ensure
  uniqueness.

###### Line

- `nosplit`: Do not split this line up into words.
  The line will not be considered a part of the gloss.
  `nosplit` lines cannot come in between regular gloss lines -- they must all
  come at the beginning and/or the end of the gloss.

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
- Small caps is indicated by surrounding the text with `^`.
- A generic `<span>` element is indicated by surrounding the text with `` ` ``.

In each of these cases, parameters come directly after the closing delimiter.

Formatting elements which use different markers (e.g. emphasis (`*`) and small
caps (`^`)) can be freely nested.
However, to include a formatting element directly inside another which uses the
_same_ marker (e.g. emphasis (`*`) and strong emphasis (`**`)), the inner
element must be surrounded by `{` `}`.

##### Parameters

> Note about `class`: In the case of a generic span, this defaults to
> `conlang`.
> Otherwise, defaults to none.

#### Text replacements (`:replace:`)

A list of text replacements can be defined in `:replace:` block.
Each replacement in the list should consist of a directive to be used for the
replacement, followed by the replacement text, which may
itself contain replacements (or any other inline formatting).
In the text, a replacement is denoted by the directive declared in the
`:replace:` block.
Text replacements do not take any parameters other than the `class` parameter.

#### Cross references (`:ref:`)

##### Parameters

- `ref`: The ID to reference in the document.
  This parameter is required.
  The text for the reference will automatically be set based on the type of
  element it refers to: "section", "table", or "gloss"; followed by the number
  of that element.
  If the reference points to an element with the `nonumber` parameter, then a
  warning will be raised, and the text will simply be the type of the element.

  This parameter can be abbreviated; the first parameter to a `:ref:` will be
  interpreted as a `ref` parameter rather than a `class` parameter.

#### External links (`:link:`)

##### Parameters

- `url`: The URL to link to.
  This parameter is required.

  This parameter can be abbreviated; the first parameter to a `:link:` will be
  interpreted as a `url` parameter rather than a `class` parameter.
- `title`: The text to display for the link.
  Defaults to the value of the `url` parameter.
