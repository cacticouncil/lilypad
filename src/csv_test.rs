use antlr_rust::common_token_stream::CommonTokenStream;
use antlr_rust::input_stream::InputStream;
use antlr_rust::tree::{ParseTree, ParseTreeListener};

use crate::antlr::csvlexer::CSVLexer;
use crate::antlr::csvlistener::CSVListener;
use crate::antlr::csvparser::*;

type Row = Vec<String>;

#[derive(Debug)]
struct CSV {
    header: Row,
    rows: Vec<Row>,
}

struct Listener {
    csv: Box<CSV>,
}

impl Listener {
    fn hdr(&self, ctx: &HdrContextAll) -> Row {
        let row_ctx = ctx.row().unwrap();
        self.row(&row_ctx)
    }

    fn row(&self, ctx: &RowContextAll) -> Row {
        let mut row = Row::new();
        let field_ctx_list = ctx.field_all();
        for (_i, field_ctx) in field_ctx_list.iter().enumerate() {
            let field = self.field(&field_ctx);
            row.push(field);
        }
        row
    }

    fn field(&self, ctx: &FieldContextAll) -> String {
        ctx.get_text()
    }
}

impl<'input> ParseTreeListener<'input, CSVParserContextType> for Listener {}

impl CSVListener<'_> for Listener {
    fn exit_csvFile(&mut self, ctx: &CsvFileContext) {
        let hdr_ctx = ctx.hdr().unwrap();
        let header = self.hdr(&hdr_ctx);
        self.csv.header = header;
        let row_ctx_list = ctx.row_all();
        for (_i, row_ctx) in row_ctx_list.iter().enumerate() {
            let row = self.row(&row_ctx);
            self.csv.rows.push(row);
        }
    }
}

pub fn parse_csv(input: &str) {
    let lexer = CSVLexer::new(InputStream::new(&*input));
    let token_source = CommonTokenStream::new(lexer);
    let mut parser = CSVParser::new(token_source);
    let listener_id = parser.add_parse_listener(Box::new(Listener {
        csv: Box::new(CSV {
            header: Row::new(),
            rows: Vec::new(),
        }),
    }));
    let result = parser.csvFile();
    assert!(result.is_ok());
    let listener = parser.remove_parse_listener(listener_id);
    let csv = listener.csv;
    println!("{:#?}", csv);
}
