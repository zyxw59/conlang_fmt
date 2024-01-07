use std::io::{Result as IoResult, Write};

use anyhow::Context;

use crate::blocks::{BlockCommon, BlockType, Parameter, UpdateParam};
use crate::document::Document;
use crate::errors::{ErrorKind, Result as EResult};
use crate::html;
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
        write!(w, "id=\"{}\" ", html::Encoder(&common.id))?;
        write!(w, "class=\"{}\">", html::Encoder(&common.class))?;
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
            write!(w, "<tr class=\"{}\">", html::Encoder(&row.class))?;
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
    fn reference_text(&self) -> Text {
        let mut text = Text::from("table ");
        if self.numbered {
            text.push(format!("{}", self.number));
        } else {
            text.extend(&self.title);
        }
        text
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
                write!(w, "scope=\"colgroup\" ")?;
            } else {
                write!(w, "scope=\"col\" ")?;
            }
        } else if header_col {
            write!(w, "<th ")?;
            if self.rows > 1 {
                write!(w, "scope=\"rowgroup\" ")?;
            } else {
                write!(w, "scope=\"row\" ")?;
            }
        } else {
            write!(w, "<td ")?;
        }
        if self.cols > 1 {
            write!(w, "colspan=\"{}\" ", self.cols)?;
        }
        if self.rows > 1 {
            write!(w, "rowspan=\"{}\" ", self.rows)?;
        }
        write!(w, "class=\"{}", html::Encoder(&self.class))?;
        if let Some(col) = col {
            write!(w, " {}", html::Encoder(&col.class))?;
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
                self.rows = param.1.parse::<usize>().context(ErrorKind::Parse)?;
                None
            }
            Some("cols") => {
                self.cols = param.1.parse::<usize>().context(ErrorKind::Parse)?;
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
