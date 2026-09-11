//! The table model round-trips through the printer (decision X9): a
//! generated valid `TableModel` printed as a `yaml table` fence parses back
//! to the same model without a diagnostic, and every fence of TeXSmith's
//! table corpus (`data/yaml-tables.md`) survives parse → print → parse and
//! prints to a fixed point.

use proptest::prelude::*;
use tmark_fmt::{format, Profile};
use tmark_ir::{
    Align, Block, Cell, Column, ColumnConfig, ColumnGroup, DataRow, Document, FileId, Inline,
    LeafColumn, Row, Separator, Str, Table, TableModel, TableSettings,
};
use tmark_syntax::parse;

// ------------------------------------------------------------ generators

fn text() -> impl Strategy<Value = String> {
    prop_oneof![
        Just(String::new()),
        Just("a".to_string()),
        Just("b c".to_string()),
        Just("1".to_string()),
        Just("1.10".to_string()),
        Just("x: y".to_string()),
        Just("Total".to_string()),
        Just("50 %".to_string()),
        Just("y".to_string()),
        Just("[x]".to_string()),
    ]
}

fn align() -> impl Strategy<Value = Option<Align>> {
    prop_oneof![
        Just(None),
        Just(Some(Align::Left)),
        Just(Some(Align::Center)),
        Just(Some(Align::Right)),
        Just(Some(Align::Justify)),
    ]
}

fn config() -> impl Strategy<Value = ColumnConfig> {
    (
        align(),
        prop_oneof![
            Just(None),
            Just(Some("X".to_string())),
            Just(Some("25%".to_string())),
            Just(Some("2.5cm".to_string())),
        ],
        prop_oneof![Just(None), Just(Some("quarter".to_string()))],
    )
        .prop_map(|(align, width, width_group)| ColumnConfig {
            align,
            width,
            width_group,
        })
}

fn name() -> impl Strategy<Value = String> {
    prop_oneof![
        Just("Fruit".to_string()),
        Just("FY23".to_string()),
        Just("2024".to_string()),
        Just("Q1".to_string()),
        Just("Nom du cours".to_string()),
    ]
}

fn leaf(named: bool) -> impl Strategy<Value = Column> {
    (name(), config(), any::<bool>()).prop_map(move |(name, config, unnamed)| {
        Column::Leaf(LeafColumn {
            name: (named || !unnamed).then_some(name),
            config,
        })
    })
}

fn column() -> impl Strategy<Value = Column> {
    prop_oneof![
        3 => leaf(false),
        2 => (name(), config(), prop::collection::vec(leaf(true), 1..=3)).prop_map(
            |(name, config, columns)| Column::Group(ColumnGroup { name, columns, config })
        ),
        1 => (
            name(),
            config(),
            prop::collection::vec(
                (name(), prop::collection::vec(leaf(true), 1..=2)).prop_map(|(name, columns)| {
                    Column::Group(ColumnGroup {
                        name,
                        columns,
                        config: ColumnConfig::default(),
                    })
                }),
                1..=2
            )
        )
            .prop_map(|(name, config, columns)| Column::Group(ColumnGroup {
                name,
                columns,
                config
            })),
    ]
}

fn settings() -> impl Strategy<Value = TableSettings> {
    (
        prop_oneof![
            Just("auto".to_string()),
            Just("100%".to_string()),
            Just("X".to_string()),
            Just("12cm".to_string()),
        ],
        prop_oneof![
            Just(None),
            Just(Some("htbp".to_string())),
            Just(Some("H".to_string()))
        ],
        prop_oneof![Just(None), Just(Some(true)), Just(Some(false))],
    )
        .prop_map(|(width, placement, long)| TableSettings {
            width,
            placement,
            long,
        })
}

/// A raw row decision per leaf: the text, and whether to open a span
/// (`cols` extra columns to the right, `rows: 2` when allowed).
type RowSeed = Vec<(String, u8, bool, Option<Align>)>;

fn seeds(
    leaves: usize,
    rows: std::ops::RangeInclusive<usize>,
) -> impl Strategy<Value = Vec<(RowSeed, bool, bool)>> {
    prop::collection::vec(
        (
            prop::collection::vec((text(), 0..3u8, any::<bool>(), align()), leaves),
            any::<bool>(),
            any::<bool>(),
        ),
        rows,
    )
}

fn cell(text: &str) -> Cell {
    if text.is_empty() {
        Cell::default()
    } else {
        Cell::new(vec![Inline::Str(Str {
            meta: Default::default(),
            text: text.to_string(),
        })])
    }
}

/// Lay the seeds into a dense, valid leaf matrix: a span never crosses an
/// absorbed slot, a row span lasts two data rows, a separator never falls
/// inside one. A row is named only when it can be spelled by name.
fn build(
    columns: Vec<Column>,
    settings: TableSettings,
    seeds: Vec<(RowSeed, bool, bool)>,
) -> TableModel {
    let model = TableModel {
        settings,
        columns,
        rows: Vec::new(),
        footer: Vec::new(),
    };
    let spans = model.top_level_spans();
    let leaves = model.leaf_count();
    let column_of = |pos: usize| spans.iter().position(|(s, l)| pos >= *s && pos < s + l);
    let names: Vec<Option<&str>> = model.columns.iter().skip(1).map(Column::name).collect();
    let nameable = spans.len() >= 2
        && spans[0].1 == 1
        && names.iter().all(Option::is_some)
        && names
            .iter()
            .enumerate()
            .all(|(i, n)| !names[..i].contains(n));
    let mut rows: Vec<Row> = Vec::new();
    let mut carried: Vec<bool> = vec![false; leaves];
    let count = seeds.len();
    for (index, (seed, separator, want_named)) in seeds.into_iter().enumerate() {
        let last = index + 1 == count;
        if separator && !carried.iter().any(|c| *c) && !rows.is_empty() {
            rows.push(Row::Separator(Separator {
                label: Some("Section".to_string()),
                double_rule: index % 2 == 0,
            }));
        }
        let mut cells: Vec<Cell> = Vec::with_capacity(leaves);
        let mut next: Vec<bool> = vec![false; leaves];
        let mut spans_cross_columns = false;
        // The top-level column whose leaves the printer writes as a list
        // (opened by a plain cell with leaves left): a span inside it
        // cannot cross the column's end.
        let mut list_open: Option<usize> = None;
        let mut pos = 0;
        while pos < leaves {
            if carried[pos] {
                cells.push(Cell::absorbed());
                pos += 1;
                continue;
            }
            let (text, extra, down, align) = &seed[pos];
            let column = column_of(pos).expect("a leaf belongs to a column");
            let column_end = spans[column].0 + spans[column].1;
            if list_open != Some(column) {
                list_open = None;
            }
            // A column span stops at the next absorbed slot and the table edge.
            let mut cols = 1;
            while cols <= *extra as usize && pos + cols < leaves && !carried[pos + cols] {
                cols += 1;
            }
            if list_open.is_some() {
                cols = cols.min(column_end - pos);
            }
            let down = *down && !last;
            if cols == 1 && !down && align.is_none() && column_end - pos > 1 {
                list_open = Some(column);
            }
            let mut c = cell(text);
            c.cols = cols as u32;
            c.rows = if down { 2 } else { 1 };
            c.align = *align;
            if column_of(pos) != column_of(pos + cols - 1) {
                spans_cross_columns = true;
            }
            cells.push(c);
            for extra in 1..cols {
                cells.push(Cell::absorbed());
                if down {
                    next[pos + extra] = true;
                }
            }
            if down {
                next[pos] = true;
            }
            pos += cols;
        }
        let named = want_named
            && nameable
            && !spans_cross_columns
            && !cells[0].absorbed
            && cells[0].rows == 1
            && cells[0].cols == 1
            && cells[0].align.is_none();
        rows.push(Row::Data(DataRow { cells, named }));
        carried = next;
    }
    TableModel { rows, ..model }
}

fn model() -> impl Strategy<Value = TableModel> {
    (prop::collection::vec(column(), 2..=4), settings())
        .prop_flat_map(|(columns, settings)| {
            let leaves: usize = columns.iter().map(Column::leaf_count).sum();
            (Just(columns), Just(settings), seeds(leaves, 1..=4))
        })
        .prop_map(|(columns, settings, seeds)| build(columns, settings, seeds))
}

// ---------------------------------------------------------------- tests

proptest! {
    #![proptest_config(ProptestConfig::with_cases(300))]

    #[test]
    fn generated_models_round_trip(model in model()) {
        let doc = Document {
            blocks: vec![Block::Table(Table {
                meta: Default::default(),
                model: model.clone(),
                attrs: Default::default(),
                source: None,
            })],
            ..Document::default()
        };
        let printed = format(&doc, Profile::Canonical);
        let parsed = parse(&printed, FileId::default());
        prop_assert!(
            parsed.diagnostics.is_empty(),
            "diagnostics on the printed table:\n{printed}\n{:?}",
            parsed.diagnostics
        );
        prop_assert_eq!(parsed.document.blocks.len(), 1, "{}", printed);
        let Block::Table(back) = &parsed.document.blocks[0] else {
            return Err(TestCaseError::fail(format!("not a table:\n{printed}")));
        };
        prop_assert!(back.source.is_none(), "{}", printed);
        prop_assert_eq!(&back.model, &model, "{}", printed);
        let again = format(&parsed.document, Profile::Canonical);
        prop_assert_eq!(&again, &printed);
    }
}

#[test]
fn texsmith_corpus_round_trips() {
    let text = include_str!("data/yaml-tables.md");
    let first = parse(text, FileId::default()).document;
    let tables: Vec<&Table> = first
        .blocks
        .iter()
        .filter_map(|b| match b {
            Block::Table(t) => Some(t),
            _ => None,
        })
        .collect();
    let configs = first
        .blocks
        .iter()
        .filter(|b| matches!(b, Block::TableConfig(_)))
        .count();
    let rejected = tables.iter().filter(|t| t.source.is_some()).count();
    // 13 + 8 `yaml table` fences (3 + 1 deliberate errors), 3 table-config.
    assert_eq!((tables.len(), rejected, configs), (21, 4, 3));
    let printed = format(&first, Profile::Canonical);
    let second = parse(&printed, FileId::default()).document;
    assert_eq!(
        tmark_ir::structural_json(&first),
        tmark_ir::structural_json(&second),
        "parse(format(corpus)) differs from the corpus"
    );
    assert_eq!(
        format(&second, Profile::Canonical),
        printed,
        "format is not a fixed point on the corpus"
    );
    // A rejected fence prints back as typed: `~` and comments included.
    assert!(printed.contains("  - [x, 1, 2]   # missing a cell\n"));
    // An accepted grouped table prints its group cells as lists, its
    // settings and width groups.
    assert!(printed.contains("  - [Apples, [120, 135, 150, 140], [130, 145, 160, 150]]\n"));
    assert!(printed.contains("table:\n  width: 100%\n"));
    assert!(printed.contains("width-group: quarter"));
}
