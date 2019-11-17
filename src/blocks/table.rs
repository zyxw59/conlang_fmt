use std::io::{Result as IoResult, Write};

use failure::ResultExt;
use htmlescape::encode_minimal_w;

use crate::blocks::{BlockCommon, BlockType, Parameter, UpdateParam};
use crate::document::{write_attribute, Document};
use crate::errors::{ErrorKind, Result as EResult};
use crate::text::{Referenceable, Text};

type OResult<T> = EResult<Option<T>>;

#[derive(Debug, Eq, PartialEq)]
pub struct Table {
    pub title: Text,
    pub numbered: bool,
    pub number: usize,
    pub rows: Vec<Row>,
    pub columns: Vec<Column>,
}

impl Table {
    pub fn new() -> Table {
        Default::default()
    }
}

impl BlockType for Table {
    fn write(
        &self,
        mut w: &mut dyn Write,
        common: &BlockCommon,
        document: &Document,
    ) -> IoResult<()> {
        write!(w, "<table ")?;
        write_attribute(&mut w, "id", &common.id)?;
        write_attribute(&mut w, "class", &common.class)?;
        writeln!(w, ">")?;
        write!(w, "<caption>")?;
        write!(w, r#"<span class="table-heading-prefix">Table"#)?;
        if self.numbered {
            write!(w, " {}", self.number)?;
        }
        write!(w, ":</span> ")?;
        self.title.write_inline(w, document)?;
        writeln!(w, "</caption>")?;
        // for recording when a cell is a continuation from an earlier row, to correctly count
        // columns
        let mut continuation_cells = Vec::<usize>::with_capacity(self.columns.len());
        for row in &self.rows {
            write!(w, "<tr ")?;
            write_attribute(&mut w, "class", &row.class)?;
            write!(w, ">")?;
            let mut col = 0;
            for cell in &row.cells {
                // increment col until we get to a free column
                while let Some(n) = continuation_cells.get_mut(col) {
                    if *n > 0 {
                        // decrement n while we're at it.
                        *n -= 1;
                        col += 1;
                    } else {
                        break;
                    }
                }
                // update continuation_cells if this cell has rowspan or colspan greater than 1
                // first, resize `continuation_cells` so that it can hold all the columns.
                if continuation_cells.len() < col + cell.cols {
                    continuation_cells.resize(col + cell.cols, 0);
                }
                for n in &mut continuation_cells[col..col + cell.cols] {
                    *n = cell.rows.max(*n).saturating_sub(1);
                }
                cell.write(&mut w, row, self.columns.get(col), document)?;
                col += cell.cols;
            }
            writeln!(w, "</tr>")?;
        }
        writeln!(w, "</table>\n")
    }

    fn update_param(&mut self, param: Parameter) -> OResult<Parameter> {
        Ok(match param.0.as_ref() {
            Some(_) => Some(param),
            None => match param.1.as_ref() {
                "nonumber" => {
                    self.numbered = false;
                    None
                }
                _ => Some(param),
            },
        })
    }

    fn as_mut_table(&mut self) -> Option<&mut Table> {
        Some(self)
    }

    fn as_referenceable(&self) -> Option<&dyn Referenceable> {
        Some(self)
    }
}

impl Referenceable for Table {
    fn write_reference(&self, w: &mut dyn Write, document: &Document) -> IoResult<()> {
        if self.numbered {
            write!(w, "table {}", self.number)?;
        } else {
            write!(w, "table ")?;
            self.title.write_inline(w, document)?;
        }
        Ok(())
    }
}

impl Default for Table {
    fn default() -> Table {
        Table {
            title: Default::default(),
            numbered: true,
            number: 0,
            rows: Default::default(),
            columns: Default::default(),
        }
    }
}

#[derive(Debug, Default, Eq, PartialEq)]
pub struct Row {
    pub cells: Vec<Cell>,
    pub header: bool,
    pub class: String,
}

impl Row {
    pub fn new() -> Row {
        Default::default()
    }
}

impl UpdateParam for Row {
    fn update_param(&mut self, param: Parameter) -> OResult<Parameter> {
        Ok(match param.0.as_ref().map(|n| n.as_ref()) {
            Some("class") => {
                self.class = param.1;
                None
            }
            None => {
                match param.1.as_ref() {
                    "header" => self.header = true,
                    _ => self.class = param.1,
                }
                None
            }
            Some(_) => Some(param),
        })
    }
}

#[derive(Debug, Default, Eq, PartialEq)]
pub struct Column {
    pub header: bool,
    pub class: String,
}

impl Column {
    pub fn new() -> Column {
        Default::default()
    }
}

impl UpdateParam for Column {
    fn update_param(&mut self, param: Parameter) -> OResult<Parameter> {
        Ok(match param.0.as_ref().map(|n| n.as_ref()) {
            Some("class") => {
                self.class = param.1;
                None
            }
            None => {
                match param.1.as_ref() {
                    "header" => self.header = true,
                    _ => self.class = param.1,
                }
                None
            }
            Some(_) => Some(param),
        })
    }
}

#[derive(Debug, Eq, PartialEq)]
pub struct Cell {
    pub rows: usize,
    pub cols: usize,
    pub class: String,
    pub text: Text,
}

impl Cell {
    pub fn new() -> Cell {
        Default::default()
    }

    fn write(
        &self,
        w: &mut impl Write,
        row: &Row,
        col: Option<&Column>,
        document: &Document,
    ) -> IoResult<()> {
        let header_row = row.header;
        let header_col = col.map(|col| col.header).unwrap_or(false);
        if header_row {
            write!(w, "<th ")?;
            if self.cols > 1 {
                write_attribute(w, "scope", "colgroup")?;
            } else {
                write_attribute(w, "scope", "col")?;
            }
        } else if header_col {
            write!(w, "<th ")?;
            if self.rows > 1 {
                write_attribute(w, "scope", "rowgroup")?;
            } else {
                write_attribute(w, "scope", "row")?;
            }
        } else {
            write!(w, "<td ")?;
        }
        if self.cols > 1 {
            write_attribute(w, "colspan", &format!("{}", self.cols))?;
        }
        if self.rows > 1 {
            write_attribute(w, "rowspan", &format!("{}", self.rows))?;
        }
        write!(w, r#"class=""#)?;
        encode_minimal_w(&self.class, w)?;
        if let Some(col) = col {
            write!(w, " ")?;
            encode_minimal_w(&col.class, w)?;
        }
        write!(w, r#"">"#)?;
        self.text.write_inline(w, document)?;
        if header_row || header_col {
            write!(w, "</th>")?;
        } else {
            write!(w, "</td>")?;
        }
        Ok(())
    }
}

impl UpdateParam for Cell {
    fn update_param(&mut self, param: Parameter) -> OResult<Parameter> {
        Ok(match param.0.as_ref().map(|n| n.as_ref()) {
            Some("class") | None => {
                self.class = param.1;
                None
            }
            Some("rows") => {
                self.rows = param
                    .1
                    .parse::<usize>()
                    .with_context(|_| ErrorKind::Parse)?;
                None
            }
            Some("cols") => {
                self.cols = param
                    .1
                    .parse::<usize>()
                    .with_context(|_| ErrorKind::Parse)?;
                None
            }
            Some(_) => Some(param),
        })
    }
}

impl Default for Cell {
    fn default() -> Cell {
        Cell {
            rows: 1,
            cols: 1,
            class: Default::default(),
            text: Default::default(),
        }
    }
}
