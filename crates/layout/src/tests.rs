Warning: truncated output (original token count: 192672)
Total output lines: 15254

use super::*;
use wordcraft_doc::fill::Fill;
use wordcraft_doc::para::InlineObject;
use wordcraft_doc::props::{Align, Border, BorderStyle, Borders, ParaProps, TextDirection};
use wordcraft_doc::{Pos, Table};

mod autospace;
mod bidi;
mod budget;
mod cell_floats;
mod complex;
mod float_top;
mod grid;
mod ligature;
pub(crate) mod linear;
mod missing_glyphs;
mod nbsp;
mod nested_cant_split;
mod numeral;
mod pages;
mod styleref;
mod table_contact;
mod textbox;
mod vertical;

const WRAP_TEST_FONT: &str = "Source Sans 3";

fn lay(doc: &Document) -> DocLayout {
    let mut c = LayoutCache::new();
    layout(doc, &mut c, &LayoutOptions::default())
}

/// These pagination fixtures test physical boundaries, not the host's Aptos substitution.
/// Keep their explicit spacing local; ordinary document/layout defaults remain unchanged.
pub(super) fn pagination_fixture_document(text: &str) -> Document {
    let mut doc = Document::from_text(text);
    Arc::make_mut(&mut doc.styles).default_chr.font = Some(WRAP_TEST_FONT.into());
    Arc::make_mut(&mut doc.styles).default_chr.size = Some(12.0);
    for block in &mut doc.body {
        if let Block::Para(para) = Arc::make_mut(block) {
            para.props.line_spacing = Some(wordcraft_doc::props::LineSpacing::Exactly(14.0));
            para.props.space_before = Some(0.0);
            para.props.space_after = Some(8.0);
        }
    }
    doc
}

/// Asserts that every laid-out fixture paragraph uses the bundled Source Sans 3 Regular bytes at
/// 12 pt and that at least one paragraph was actually inspected.
pub(super) fn assert_pagination_fixture_font(layout: &DocLayout) {
    let mut paragraphs = 0;
    for page in &layout.pages {
        for item in &page.items {
            if let Placed::Lines { para, .. } = item {
                paragraphs += 1;
                for style in &para.styles {
                    assert_eq!(style.rc.font, WRAP_TEST_FONT, "the fixture requests its bundled face");
                    assert_eq!(style.face.family, WRAP_TEST_FONT, "the fixture must not silently substitute another family");
                    assert_eq!(style.face.style, "Regular", "the fixture must resolve the intended regular face");
                    assert_eq!(style.size, 12.0, "the fixture keeps its explicit point size");
                    assert!(
                        style.face.data() == include_bytes!("../../../assets/fonts/SourceSans3-Regular.ttf").as_slice(),
                        "the selected face must be the exact already-bundled regular font bytes"
                    );
                }
            }
        }
    }
    assert!(paragraphs > 0, "font assertions must inspect actual laid-out paragraphs");
}

/// Preserve the 22 short filler paragraphs while giving the following row a measured boundary.
fn pagination_filler_leaving(room: f32) -> Document {
    let lines: Vec<String> = (1..=22).map(|i| format!("Filler {i}")).collect();
    let mut doc = pagination_fixture_document(&lines.join("\n"));
    let after = doc.last_section.text_height() - room - 22.0 * 14.0 - 21.0 * 8.0;
    assert!(after >= 8.0, "the last gap may add room-taking space, not remove filler content");
    doc.para_mut(StoryRef::Body, &Path::top(21)).unwrap().props.space_after = Some(after);
    let probe = lay(&doc);
    assert_pagination_fixture_font(&probe);
    assert_eq!(probe.pages.len(), 1, "all filler must precede the row on page one");
    let lines: Vec<_> = probe.pages[0]
        .items
        .iter()
        .filter_map(|it| match it {
            Placed::Lines { story: StoryRef::Body, path, para, y, l0, l1, .. } if path.0.len() == 1 => Some((para, *y, l1 - l0)),
            _ => None,
        })
        .collect();
    assert_eq!(lines.len(), 22, "every filler paragraph remains");
    assert!(lines.iter().all(|(_, _, count)| *count == 1), "filler text must remain one line per paragraph");
    let (last, y, _) = lines.last().unwrap();
    let remaining = probe.pages[0].body.bottom() - y - last.height - last.rp.space_after;
    assert!((remaining - room).abs() < 0.02, "requested {room}pt of actual row room, got {remaining}pt");
    assert!(remaining > 0.0 && (remaining < 63.6) == (room < 63.6), "the measured room must be on the intended side of the row boundary");
    doc
}

/// The top-level body text bottom and its trailing space-after, if the page has one.
fn pagination_body_bottom(page: &Page) -> Option<(f32, f32)> {
    let mut body_bottom = None::<(f32, f32)>;
    for item in &page.items {
        if let Placed::Lines { story: StoryRef::Body, path, para, l0, l1, y, .. } = item
            && path.0.len() == 1
            && let (Some(first), Some(last)) = (para.lines.get(*l0), para.lines.get(l1.saturating_sub(1)))
        {
            let bottom = y + last.top + last.height - first.top;
            if body_bottom.is_none_or(|(prior, _)| bottom > prior) {
                body_bottom = Some((bottom, para.rp.space_after));
            }
        }
    }
    body_bottom
}

/// Report the page's top-level body text bottom, when there is one.
fn pagination_write_body_bottom(out: &mut String, page: &Page) {
    use std::fmt::Write as _;
    if let Some((bottom, after)) = pagination_body_bottom(page) {
        let _ = writeln!(
            out,
            "  top-level body text bottom={bottom:.3} trailing_space_after={after:.3} remaining_to_body_bottom={:.3} remaining_after_unadjusted_space={:.3}",
            page.body.bottom() - bottom,
            page.body.bottom() - bottom - after
        );
    }
}

/// The first and last line of the placed range, with their physical extents.
fn pagination_write_line_rows(out: &mut String, para: &ParaLayout, l0: usize, l1: usize) {
    use std::fmt::Write as _;
    for (li, line) in para.lines.iter().enumerate().filter(|(i, _)| *i >= l0 && *i < l1 && (*i == l0 || i.saturating_add(1) == l1)) {
        let _ = writeln!(
            out,
            "    line={li} bytes={}..{} top={:.3} height={:.3} baseline={:.3} text_asc={:.3} text_desc={:.3} end={:?}",
            line.start, line.stop, line.top, line.height, line.baseline, line.text_asc, line.text_desc, line.end
        );
    }
}

/// The styles this paragraph actually uses (plus the first), with their resolved metrics.
fn pagination_write_styles(out: &mut String, para: &ParaLayout) {
    use std::fmt::Write as _;
    for (si, st) in para.styles.iter().enumerate().filter(|(i, _)| *i == 0 || para.clusters.iter().any(|c| c.style as usize == *i)).take(4) {
        let (asc, desc) = wordcraft_fonts::word::line_metrics(&st.face);
        let raw_height = (asc + desc) * f64::from(st.size) / st.face.upem;
        let _ = writeln!(
            out,
            "    style={si} requested={:?} actual={:?}/{:?} face_id={} upem={} size={} word_asc={} word_desc={} raw_height_pt={raw_height:.3} resolved_asc={:.3} resolved_desc={:.3}",
            st.rc.font,
            st.face.family,
            st.face.style,
            st.face.id(),
            st.face.upem,
            st.size,
            asc,
            desc,
            st.ascent,
            st.descent
        );
    }
}

/// One placed paragraph line range and its styles.
fn pagination_write_lines(out: &mut String, ii: usize, item: &Placed, doc: Option<&Document>) {
    use std::fmt::Write as _;
    let Placed::Lines { story, path, para, l0, l1, x, y, turn } = item else {
        return;
    };
    let text = doc.and_then(|d| d.para(*story, path)).map(|p| p.text.chars().take(96).collect::<String>());
    let _ = writeln!(
        out,
        "  item={ii} Lines story={story:?} path={path:?} text={text:?} xy=({x:.3},{y:.3}) turn={turn:?} lines={l0}..{l1}/{} para_height={:.3} before={:.3} after={:.3} spacing={:?}",
        para.lines.len(),
        para.height,
        para.rp.space_before,
        para.rp.space_after,
        para.rp.line_spacing
    );
    pagination_write_line_rows(out, para, *l0, *l1);
    pagination_write_styles(out, para);
}

/// One retained placed item (the middle items are omitted by the caller).
fn pagination_write_item(out: &mut String, ii: usize, item: &Placed, doc: Option<&Document>) {
    use std::fmt::Write as _;
    match item {
        Placed::Lines { .. } => pagination_write_lines(out, ii, item, doc),
        Placed::Cell { rect, table, row, cell, story, clip, .. } => {
            let _ = writeln!(out, "  item={ii} Cell table={table:?} story={story:?} row={row} cell={cell} rect={rect:?} clip={clip}");
        }
        Placed::Object { rect, story, path, off, text_box, .. } => {
            let _ = writeln!(out, "  item={ii} Object story={story:?} path={path:?} off={off} textbox={text_box:?} rect={rect:?}");
        }
        _ => {}
    }
}

/// Failure-only evidence for the selected pagination fixtures below. Inspect the already produced
/// styles and placements, not a second run.
pub(super) fn pagination_fixture_diagnostics(label: &str, doc: Option<&Document>, layout: &DocLayout) -> String {
    use std::fmt::Write as _;
    let mut out = String::new();
    let _ = writeln!(out, "\npagination fixture {label}: {} pages", layout.pages.len());
    for (pi, page) in layout.pages.iter().enumerate().take(8) {
        let _ = writeln!(
            out,
            "page={pi} body={:?} first_block={} floats={:?} items={}",
            page.body,
            page.first_block,
            page.float_tables,
            page.items.len()
        );
        pagination_write_body_bottom(&mut out, page);
        for (ii, item) in page.items.iter().enumerate().filter(|(i, _)| *i < 32 || i.saturating_add(16) >= page.items.len()) {
            pagination_write_item(&mut out, ii, item, doc);
        }
        if page.items.len() > 48 {
            let _ = writeln!(out, "  omitted {} middle items (first32/last16 retained)", page.items.len() - 48);
        }
    }
    if layout.pages.len() > 8 {
        let _ = writeln!(out, "omitted {} later pages", layout.pages.len() - 8);
    }
    out
}

#[derive(Clone, Copy)]
enum SpacingLocation {
    Cell,
    Body,
    Header,
}

fn legacy_table_gap(mode: u32, allow: bool, location: SpacingLocation) -> f32 {
    let mut d = Document::new();
    d.settings.compat_mode = mode;
    d.settings.allow_space_of_same_style_in_table = allow;
    let chr = wordcraft_doc::CharProps { font: Some("Source Sans 3".into()), size: Some(12.0), ..Default::default() };
    let mut a = wordcraft_doc::Paragraph::with_text("Alpha probe", chr.clone());
    a.props = ParaProps {
        space_before: Some(0.0),
        space_after: Some(12.0),
        line_spacing: Some(wordcraft_doc::props::LineSpacing::Exactly(24.0)),
        contextual_spacing: Some(true),
        ..Default::default()
    };
    let mut b = wordcraft_doc::Paragraph::with_text("Beta probe", chr);
    b.props = ParaProps { space_after: Some(0.0), ..a.props.clone() };
    let blocks = vec![wordcraft_doc::para_block(a), wordcraft_doc::para_block(b)];
    match location {
        SpacingLocation::Cell => {
            let mut table = Table::new(1, 1, 468.0);
            // Cell identity must not depend on whether table-style formatting exists.
            table.props.style = None;
            table.props.cell_margins = Some([0.0; 4]);
            table.rows[0].cells[0].blocks = blocks;
            d.body = vec![std::sync::Arc::new(wordcraft_doc::Block::Table(table))];
        }
        SpacingLocation::Body => d.body = blocks,
        SpacingLocation::Header => {
            let id = d.add_part(wordcraft_doc::PartKind::Header, blocks);
            d.last_section.headers.default = Some(id);
        }
    }
    let l = lay(&d);
    let items = if matches!(location, SpacingLocation::Header) { &l.pages[0].header } else { &l.pages[0].items };
    let baselines: Vec<_> = items
        .iter()
        .filter_map(|i| match i {
            Placed::Lines { para, y, .. } => Some(y + para.lines[0].baseline),
            _ => None,
        })
        .collect();
    assert_eq!(baselines.len(), 2);
    baselines[1] - baselines[0] - 24.0
}

#[test]
fn legacy_table_contextual_spacing_flag_keeps_after_space() {
    crate::test_fonts();
    // Original Word for Mac 16.113.3 probe: after 12 pt, before 0 pt, both contextual.
    // Zero before-space keeps the compatibility rule independent of ordinary overlap.
    for mode in [11, 12, 14] {
        let gap = legacy_table_gap(mode, true, SpacingLocation::Cell);
        assert!((gap - 12.0).abs() < 0.01, "mode={mode}: gap={gap}, expected 12 pt");
    }
}

#[test]
fn legacy_table_contextual_spacing_flag_is_scoped_to_cells_and_legacy_mode() {
    crate::test_fonts();
    for (mode, allow) in [(11, false), (12, false), (14, false), (15, false), (15, true)] {
        assert!(legacy_table_gap(mode, allow, SpacingLocation::Cell).abs() < 0.01);
    }
    for mode in [14, 15] {
        for location in [SpacingLocation::Body, SpacingLocation::Header] {
            assert!(legacy_table_gap(mode, true, location).abs() < 0.01);
        }
    }
}

/// Baseline of the paragraph after a one-row table, less that of the first cell's last
/// paragraph, which has 12 pt after in `style`, contextual or not. `next_cell` adds a second cell
/// whose paragraph has that style; `after_table` styles the paragraph after the table; `text` is
/// the last paragraph's text.
fn cell_end_gap(style: &str, contextual: bool) -> f32 {
    cell_end_gap_in(Document::new(), style, contextual, None, None, "Beta probe")
}

fn cell_end_gap_in(mut d: Document, style: &str, contextual: bool, next_cell: Option<&str>, after_table: Option<&str>, text: &str) -> f32 {
    use wordcraft_doc::props::LineSpacing;
    Arc::make_mut(&mut d.styles).upsert(wordcraft_doc::Style {
        id: "Other".into(),
        name: "Other".into(),
        based_on: Some("Normal".into()),
        ..Default::default()
    });
    let chr = wordcraft_doc::CharProps { font: Some("Source Sans 3".into()), size: Some(12.0), ..Default::default() };
    let props = ParaProps { space_before: Some(0.0), space_after: Some(0.0), line_spacing: Some(LineSpacing::Exactly(24.0)), ..Default::default() };
    let para = |text: &str, props: ParaProps| {
        let mut p = wordcraft_doc::Paragraph::with_text(text, chr.clone());
        p.props = props;
        wordcraft_doc::para_block(p)
    };
    let styled = |s: Option<&str>| ParaProps { style: s.map(Into::into), ..props.clone() };
    let mut table = Table::new(1, if next_cell.is_some() { 2 } else { 1 }, 468.0);
    table.props.style = None;
    table.props.cell_margins = Some([0.0; 4]);
    let last = ParaProps { style: Some(style.into()), space_after: Some(12.0), contextual_spacing: Some(contextual), ..props.clone() };
    table.rows[0].cells[0].blocks = vec![para("Alpha probe", props.clone()), para(text, last)];
    if let Some(cell) = table.rows[0].cells.get_mut(1) {
        cell.blocks = vec![para("Delta probe", styled(next_cell))];
    }
    d.body = vec![std::sync::Arc::new(wordcraft_doc::Block::Table(table)), para("Gamma probe", styled(after_table))];
    let l = lay(&d);
    let baseline = |at: &[u32]| {
        l.pages[0].items.iter().find_map(|i| match i {
            Placed::Lines { para, y, path, .. } if path.0 == at => Some(y + para.lines[0].baseline),
            _ => None,
        })
    };
    let (Some(cell), Some(after)) = (baseline(&[0, 0, 0, 1]), baseline(&[1])) else { panic!("probe lines missing") };
    after - cell
}

#[test]
fn contextual_spacing_drops_the_space_after_a_cells_last_normal_paragraph() {
    // The end-of-cell mark is a Normal paragraph: contextual spacing drops the 12 pt after a
    // Normal last paragraph (docxide-pdf `cell-end-text-context`, `cell-end-empty-context`), not
    // after one in another style (`cell-end-other-style`) or without contextual spacing.
    let kept = cell_end_gap("Normal", false);
    assert!((cell_end_gap("Other", true) - kept).abs() < 0.01);
    let dropped = cell_end_gap("Normal", true);
    assert!((kept - dropped - 12.0).abs() < 0.01, "kept {kept}, dropped {dropped}");
    // An undefined style is formatted as Normal, and drops it too.
    assert!((cell_end_gap("Missing", true) - dropped).abs() < 0.01);
    // The mark decides, not a neighbour: another style in the next cell or after the table
    // leaves the drop as it is.
    for (next_cell, after_table) in [(Some("Other"), None), (None, Some("Other"))] {
        let gap = cell_end_gap_in(Document::new(), "Normal", true, next_cell, after_table, "Beta probe");
        assert!((gap - dropped).abs() < 0.01, "{next_cell:?} {after_table:?}: {gap}, dropped {dropped}");
    }
    // And the reverse: another style keeps its space beside Normal neighbours.
    let gap = cell_end_gap_in(Document::new(), "Other", true, Some("Normal"), Some("Normal"), "Beta probe");
    assert!((gap - kept).abs() < 0.01, "other beside Normal: {gap}, kept {kept}");
    // An empty last paragraph drops it too (`cell-end-empty-context`).
    let gap = cell_end_gap_in(Document::new(), "Normal", true, None, None, "");
    assert!((gap - dropped).abs() < 0.01, "empty: {gap}, dropped {dropped}");
    // Where `allowSpaceOfSameStyleInTable` keeps same-style space in cells (modes 11/12/14), the
    // 12 pt stays.
    let mut legacy = Document::new();
    legacy.settings.compat_mode = 14;
    legacy.settings.allow_space_of_same_style_in_table = true;
    let gap = cell_end_gap_in(legacy, "Normal", true, None, None, "Beta probe");
    assert!((gap - kept).abs() < 0.01, "legacy {gap}, kept {kept}");
}

fn contextual_gap(after: f32, before: f32, first: bool, second: bool, different_style: bool, header: bool) -> f32 {
    use wordcraft_doc::props::LineSpacing;
    let mut d = Document::new();
    for id in ["SpacingA", "SpacingB"] {
        Arc::make_mut(&mut d.styles).upsert(wordcraft_doc::Style { id: id.into(), name: id.into(), ..Default::default() });
    }
    let chr = wordcraft_doc::CharProps { font: Some("Source Sans 3".into()), size: Some(12.0), ..Default::default() };
    let props = ParaProps {
        style: Some("SpacingA".into()),
        space_before: Some(0.0),
        space_after: Some(0.0),
        line_spacing: Some(LineSpacing::Exactly(24.0)),
        ..Default::default()
    };
    let mut a = wordcraft_doc::Paragraph::with_text("Alpha probe", chr.clone());
    a.props = ParaProps { space_after: Some(after), contextual_spacing: Some(first), ..props.clone() };
    let mut b = wordcraft_doc::Paragraph::with_text("Beta probe", chr);
    b.props = ParaProps {
        style: Some(if different_style { "SpacingB" } else { "SpacingA" }.into()),
        space_before: Some(before),
        contextual_spacing: Some(second),
        ..props
    };
    let paragraphs = vec![wordcraft_doc::para_block(a), wordcraft_doc::para_block(b)];
    if header {
        let id = d.add_part(wordcraft_doc::PartKind::Header, paragraphs);
        d.last_section.headers.default = Some(id);
    } else {
        d.body = paragraphs;
    }
    let l = lay(&d);
    let items = if header { &l.pages[0].header } else { &l.pages[0].items };
    let baselines: Vec<_> =
        items.iter().filter_map(|i| if let Placed::Lines { para, y, .. } = i { Some(y + para.lines[0].baseline) } else { None }).collect();
    assert_eq!(baselines.len(), 2);
    baselines[1] - baselines[0] - 24.0
}

// Original two-paragraph probes measured in Word for Mac 16.113.3, build 26092714.
// Exactly 24 pt spacing isolates the extra gap from font-dependent line height.
#[test]
fn contextual_after_removes_a_smaller_before_gap() {
    crate::test_fonts();
    for header in [false, true] {
        let gap = contextual_gap(12.0, 8.0, true, false, false, header);
        assert!(gap.abs() < 0.01, "header={header}: gap={gap}, expected 0 pt");
    }
}

#[test]
fn contextual_after_keeps_only_the_residual_before_gap() {
    crate::test_fonts();
    // ISO/IEC 29500-1 §17.3.1.9's worked example: max(10, 12) - 10 = 2 pt.
    for header in [false, true] {
        let gap = contextual_gap(10.0, 12.0, true, false, false, header);
        assert!((gap - 2.0).abs() < 0.01, "header={header}: gap={gap}, expected 2 pt");
    }
}

#[test]
fn contextual_spacing_preserves_each_side_and_style_identity() {
    crate::test_fonts();
    for header in [false, true] {
        for (after, before, first, second, different, expected) in [
            (12.0, 8.0, false, true, false, 12.0),
            (12.0, 8.0, true, true, false, 0.0),
            (12.0, 0.0, true, true, true, 12.0),
            (12.0, 0.0, false, false, false, 12.0),
        ] {
            let gap = contextual_gap(after, before, first, second, different, header);
            assert!(
                (gap - expected).abs() < 0.01,
                "header={header}, first={first}, second={second}, different={different}: gap={gap}, expected {expected}"
            );
        }
    }
}

#[test]
fn contextual_spacing_removes_every_gap_in_a_four_paragraph_run() {
    crate::test_fonts();
    use wordcraft_doc::props::LineSpacing;
    for table in [false, true] {
        let mut doc = Document::new();
        Arc::make_mut(&mut doc.styles).upsert(wordcraft_doc::Style { id: "SpacingA".into(), name: "SpacingA".into(), ..Default::default() });
        let paragraphs = ["One", "Two", "Three", "Four"]
            .into_iter()
            .map(|text| {
                let mut p = wordcraft_doc::Paragraph::with_text(
                    text,
                    wordcraft_doc::CharProps { font: Some("Source Sans 3".into()), size: Some(12.0), ..Default::default() },
                );
                p.props = ParaProps {
                    style: Some("SpacingA".into()),
                    space_before: Some(6.0),
                    space_after: Some(6.0),
                    contextual_spacing: Some(true),
                    line_spacing: Some(LineSpacing::Exactly(24.0)),
                    ..Default::default()
                };
                wordcraft_doc::para_block(p)
            })
            .collect::<Vec<_>>();
        if table {
            let mut cell = Table::new(1, 1, 400.0);
            cell.rows[0].cells[0].blocks = paragraphs;
            doc.body = vec![Arc::new(Block::Table(cell))];
        } else {
            doc.body = paragraphs;
        }
        let layout = lay(&doc);
        let baselines = layout.pages[0]
            .items
            .iter()
            .filter_map(|item| match item {
                Placed::Lines { para, y, .. } => Some(y + para.lines[0].baseline),
                _ => None,
            })
            .collect::<Vec<_>>();
        assert_eq!(baselines.len(), 4);
        // Every same-style contextual boundary has zero extra space, including after paragraph two.
        for pair in baselines.windows(2) {
            assert!((pair[1] - pair[0] - 24.0).abs() < 0.01, "table={table}: {baselines:?}");
        }
    }
}

// Contextual and automatic list spacing can suppress the same after-space only once.
#[test]
fn contextual_automatic_lists_do_not_subtract_the_gap_twice() {
    crate::test_fonts();
    use wordcraft_doc::props::{LineSpacing, NumRef};
    for table in [false, true] {
        let mut doc = Document::new();
        let num = Arc::make_mut(&mut doc.numbering).add_list(wordcraft_doc::ListKind::Numbered);
        let paragraphs = ["First", "Second", "Third"]
            .into_iter()
            .map(|text| {
                let mut p = wordcraft_doc::Paragraph::with_text(text, Default::default());
                p.props = ParaProps {
                    contextual_spacing: Some(true),
                    space_before_auto: Some(true),
                    space_after_auto: Some(true),
                    numbering: Some(NumRef { num, level: 0 }),
                    line_spacing: Some(LineSpacing::Exactly(24.0)),
                    ..Default::default()
                };
                wordcraft_doc::para_block(p)
            })
            .collect::<Vec<_>>();
        if table {
            let mut cell = Table::new(1, 1, 400.0);
            cell.rows[0].cells[0].blocks = paragraphs;
            doc.body = vec![Arc::new(Block::Table(cell))];
        } else {
            doc.body = paragraphs;
        }
        let layout = lay(&doc);
        let baselines = layout.pages[0]
            .items
            .iter()
            .filter_map(|item| match item {
                Placed::Lines { para, y, .. } => Some(y + para.lines[0].baseline),
                _ => None,
            })
            .collect::<Vec<_>>();
        assert_eq!(baselines.len(), 3);
        for pair in baselines.windows(2) {
            assert!((pair[1] - pair[0] - 24.0).abs() < 0.01, "table={table}: {baselines:?}");
        }
    }
}

fn lines_of(l: &DocLayout) -> usize {
    l.pages.iter().flat_map(|p| p.items.iter()).map(|it| if let Placed::Lines { l0, l1, .. } = it { l1 - l0 } else { 0 }).sum()
}

#[test]
fn empty_doc_has_one_page() {
    crate::test_fonts();
    let d = Document::new();
    let l = lay(&d);
    assert_eq!(l.pages.len(), 1);
    assert_eq!(lines_of(&l), 1);
    let c = l.caret(&Pos::body(0, 0)).unwrap();
    assert_eq!(c.page, 0);
    assert!((c.x - 72.0).abs() < 0.5, "{c:?}");
    assert!(c.top >= 72.0 && c.height > 8.0);
}

#[test]
fn hidden_marks_take_no_space() {
    crate::test_fonts();
    let mut d = Document::new();
    d.body = vec![wordcraft_doc::para_block(wordcraft_doc::Paragraph::with_text("Before", Default::default()))];
    for _ in 0..20 {
        let mut p = wordcraft_doc::Paragraph::new();
        p.mark.hidden = Some(true);
        d.body.push(wordcraft_doc::para_block(p));
    }
    d.body.push(wordcraft_doc::para_block(wordcraft_doc::Paragraph::with_text("After", Default::default())));
    let l = lay(&d);
    let y = |block: usize| {
        l.pages
            .iter()
            .flat_map(|p| &p.items)
            .find_map(|item| match item {
                Placed::Lines { path, y, .. } if *path == Path::top(block) => Some(*y),
                _ => None,
            })
            .unwrap()
    };
    let mut plain = Document::new();
    plain.body = vec![
        wordcraft_doc::para_block(wordcraft_doc::Paragraph::with_text("Before", Default::default())),
        wordcraft_doc::para_block(wordcraft_doc::Paragraph::with_text("After", Default::default())),
    ];
    let plain_layout = lay(&plain);
    let plain_after = plain_layout.pages[0]
        .items
        .iter()
        .find_map(|item| match item {
            Placed::Lines { path, y, .. } if *path == Path::top(1) => Some(*y),
            _ => None,
        })
        .unwrap();
    assert!((y(21) - plain_after).abs() < 0.1, "hidden paragraphs moved the following text to {} vs {plain_after}", y(21));
    let shown = layout(&d, &mut LayoutCache::new(), &LayoutOptions { show_hidden: true, ..Default::default() });
    let after = shown.caret(&Pos::body(21, 0)).unwrap();
    assert!(after.top > plain_after + 100.0, "showing hidden marks must restore their lines");
}

#[test]
fn long_paragraph_wraps_and_paginates() {
    crate::test_fonts();
    let text = "The quick brown fox jumps over the lazy dog. ".repeat(400);
    let d = Document::from_text(&text);
    let l = lay(&d);
    assert!(l.pages.len() >= 3, "pages {}", l.pages.len());
    // Every line fits the text width.
    for p in &l.pages {
        for it in &p.items {
            if let Placed::Lines { para, l0, l1, x, .. } = it {
                for li in *l0..*l1 {
                    let line = &para.lines[li];
                    let w = para.x_of(li, line.stop).unwrap() + x;
                    assert!(w <= 612.0 - 72.0 + 30.0, "line too wide: {w}");
                }
            }
        }
    }
}

#[test]
fn reading_layout_reflows_into_viewport_height_pages_without_changing_print_layout() {
    crate::test_fonts();
    let d = Document::from_text(&"A moderately long sentence that should wrap across reading columns. ".repeat(250));
    let opts = LayoutOptions { view: ViewMode::Read, web_width: 230.0, read_height: 390.0, ..Default::default() };
    let read = layout(&d, &mut LayoutCache::new(), &opts);
    assert!(read.pages.len() > 5, "reading column count: {}", read.pages.len());
    assert!(read.pages.iter().all(|p| p.w == 230.0 && p.h == 390.0));
    assert!(read.pages.iter().all(|p| p.body.w < p.w && p.body.h < p.h));
    assert_eq!(
        lines_of(&read),
        lines_of(&layout(&d, &mut LayoutCache::new(), &LayoutOptions { view: ViewMode::Web, web_width: 230.0, ..Default::default() }))
    );
    let print = lay(&d);
    assert_eq!(print.pages[0].w, d.sections()[0].1.page_w);
    assert_eq!(print.pages[0].h, d.sections()[0].1.page_h);
    // Window height changes page breaks, not the text, and a malformed dimension stays bounded.
    let short = layout(&d, &mut LayoutCache::new(), &LayoutOptions { read_height: 260.0, ..opts.clone() });
    assert!(short.pages.len() > read.pages.len());
    let spaced = layout(&d, &mut LayoutCache::new(), &LayoutOptions { read_line_spacing: Some(2.0), ..opts.clone() });
    assert!(spaced.pages.len() > read.pages.len(), "reader-only line spacing repaginates text");
    let invalid = layout(&d, &mut LayoutCache::new(), &LayoutOptions { web_width: f32::NAN, read_height: f32::INFINITY, ..opts });
    assert!(invalid.pages.iter().all(|p| p.w.is_finite() && p.h.is_finite()));
}

#[test]
fn hit_and_caret_agree() {
    crate::test_fonts();
    let d = Document::from_text("Hello world, this is WordCraft.\nSecond paragraph here.");
    let l = lay(&d);
    for off in [0, 5, 12, 31] {
        let pos = Pos::body(0, off);
        let c = l.caret(&pos).unwrap();
        let back = l.hit(c.page, c.x + 0.1, c.top + c.height / 2.0, StoryRef::Body).unwrap();
        assert_eq!(back, pos, "off {off}");
    }
    let c1 = l.caret(&Pos::body(1, 0)).unwrap();
    let (down, _) = l.vertical(&Pos::body(0, 0), None, 1, 0).unwrap();
    assert_eq!(down.path, Path::top(1));
    assert!(c1.top > l.caret(&Pos::body(0, 0)).unwrap().top);
    let (up, _) = l.vertical(&Pos::body(1, 3), None, -1, 0).unwrap();
    assert_eq!(up.path, Path::top(0));
}

#[test]
fn selection_bar_finds_the_line_beside_the_pointer() {
    crate::test_fonts();
    // #856: the left margin beside a line is Word's selection bar.
    let text = format!("{}\nShort.", "The quick brown fox jumps over the lazy dog. ".repeat(8));
    let mut d = Document::from_text(&text);
    d.insert_block(StoryRef::Body, &Path::top(2), wordcraft_doc::Block::Table(Table::new(1, 1, 468.0))).unwrap();
    let l = lay(&d);
    let p = &l.pages[0];
    let c = l.caret(&Pos::body(0, 0)).unwrap();
    let first = l.selection_bar_at(0, 30.0, c.top + c.height / 2.0).unwrap();
    assert_eq!(first, Pos::body(0, 0));
    // The second line of the wrapped paragraph: its start, and its span is the whole line.
    let line2 = l.selection_bar_at(0, 30.0, c.top + c.height * 1.5).unwrap();
    assert_eq!(line2.path, Path::top(0));
    assert!(line2.off > 0);
    let (a, b, last) = l.line_span(&line2, 0).unwrap();
    assert_eq!(a, line2);
    assert!(b.off > a.off && !last);
    let (_, b, last) = l.line_span(&Pos::body(1, 0), 0).unwrap();
    assert_eq!(b.off, "Short.".len());
    assert!(last);
    // In the text column, the header area, off the page or beside a table: not the bar.
    assert_eq!(l.selection_bar_at(0, p.body.x + 5.0, c.top + 2.0), None);
    assert_eq!(l.selection_bar_at(0, 30.0, p.body.y - 10.0), None);
    assert_eq!(l.selection_bar_at(0, -5.0, c.top + 2.0), None);
    assert_eq!(l.selection_bar_at(0, f32::NAN, c.top), None);
    assert_eq!(l.selection_bar_at(9, 30.0, c.top), None);
    let cell = l.caret(&Pos { story: StoryRef::Body, path: Path(vec![2, 0, 0, 0]), off: 0 }).unwrap();
    assert_eq!(l.selection_bar_at(0, 30.0, cell.top + cell.height / 2.0), None);
    // Dragging takes the nearest top-level line at any height.
    assert_eq!(l.body_line_at(0, -100.0), Some(Pos::body(0, 0)));
}

#[test]
fn selection_bar_of_a_right_to_left_paragraph_is_in_the_right_margin() {
    // Paragraph 0 reads left to right, paragraph 1 right to left, paragraph 2 left to right.
    let mut d = Document::from_text("Left one\n\u{5e9}\u{5dc}\u{5d5}\u{5dd} \u{5e9}\u{5dc}\u{5d5}\u{5dd}\nLeft two");
    let at = Pos::body(1, 0);
    d.format_paragraphs(&at, &at, &|p: &mut ParaProps| p.bidi = Some(true)).unwrap();
    let l = lay(&d);
    let p = &l.pages[0];
    let mid = |i: usize| {
        let c = l.caret(&Pos::body(i, 0)).unwrap();
        c.top + c.height / 2.0
    };
    let (left_x, right_x) = (p.body.x / 2.0, (p.body.right() + p.w) / 2.0);
    // The right-to-left paragraph: the bar is on the right, not the left.
    assert_eq!(l.selection_bar_at(0, right_x, mid(1)).map(|q| q.path), Some(Path::top(1)));
    assert_eq!(l.selection_bar_at(0, left_x, mid(1)), None);
    // Left-to-right neighbours keep the left margin only (each line uses its own side).
    assert_eq!(l.selection_bar_at(0, left_x, mid(0)), Some(Pos::body(0, 0)));
    assert_eq!(l.selection_bar_at(0, right_x, mid(0)), None);
    assert_eq!(l.selection_bar_at(0, left_x, mid(2)), Some(Pos::body(2, 0)));
    assert_eq!(l.selection_bar_at(0, right_x, mid(2)), None);
    // Not in the text column, off the page, or hostile numbers.
    assert_eq!(l.selection_bar_at(0, p.body.right() - 5.0, mid(1)), None);
    assert_eq!(l.selection_bar_at(0, p.w + 5.0, mid(1)), None);
    assert_eq!(l.selection_bar_at(0, f32::INFINITY, mid(1)), None);
    // The line span is the whole line, as for the left bar.
    let q = l.selection_bar_at(0, right_x, mid(1)).unwrap();
    assert!(l.line_span(&q, 0).is_some());
}

#[test]
fn deletions_leave_the_final_text_layout() {
    crate::test_fonts();
    // "Keep " + deleted "DELETEDTEXT " (5..17) + inserted "INSERTED".
    let mut d = Document::from_text("Keep DELETEDTEXT INSERTED");
    d.format_range(&Pos::body(0, 5), &Pos::body(0, 17), &|c| c.del = Some(0)).unwrap();
    d.format_range(&Pos::body(0, 17), &Pos::body(0, 25), &|c| c.ins = Some(0)).unwrap();
    let texts = |l: &DocLayout, markup: bool| -> (String, usize) {
        let items = display::page_display(&d, &l.pages[0], &display::DisplayOptions { markup, ..Default::default() });
        let text = items.iter().filter_map(|i| if let display::Draw::Glyphs { text, .. } = i { Some(text.as_str()) } else { None }).collect();
        (text, items.iter().filter(|i| matches!(i, display::Draw::Line { .. })).count())
    };
    // Markup: the deletion is laid out and struck through, the insertion underlined, and the
    // changed line has a bar in the margin.
    let full = lay(&d);
    assert_eq!(texts(&full, true), ("Keep DELETEDTEXT INSERTED".into(), 3));
    // Without markup a full layout still never prints the deletion as plain text.
    assert!(!texts(&full, false).0.contains("DELETED"));
    // The final layout gives the deletion no width.
    let fin = layout(&d, &mut LayoutCache::new(), &LayoutOptions { hide_deleted: true, ..Default::default() });
    assert_eq!(texts(&fin, false), ("Keep INSERTED".into(), 0));
    let at = |off| fin.caret(&Pos::body(0, off)).unwrap();
    assert!((at(5).x - at(17).x).abs() < 0.01, "{:?} {:?}", at(5), at(17));
    // Every offset maps to a caret, and clicks land outside the hidden text.
    for off in 0..=25 {
        let c = at(off);
        let back = fin.hit(c.page, c.x + 0.1, c.top + c.height / 2.0, StoryRef::Body).unwrap();
        assert!(back.off <= 5 || back.off >= 17, "off {off} hit {back:?}");
    }
}

#[test]
fn deleted_soft_hyphen_is_no_hyphenation_point_without_markup() {
    crate::test_fonts();
    // "ab\u{ad}cdef" in a column where "ab-" fits but the whole word does not; only the soft
    // hyphen is track-deleted.
    let mut d = Document::from_text("ab\u{ad}cdef");
    d.format_range(&Pos::body(0, 2), &Pos::body(0, 4), &|c| c.del = Some(0)).unwrap();
    d.format_paragraphs(&Pos::body(0, 0), &Pos::body(0, 0), &|p| p.indent_right = Some(468.0 - 26.0)).unwrap();
    let hyphens = |hide_deleted: bool| {
        let l = layout(&d, &mut LayoutCache::new(), &LayoutOptions { hide_deleted, ..Default::default() });
        let Some(Placed::Lines { para, .. }) = l.pages[0].items.iter().find(|i| matches!(i, Placed::Lines { .. })) else { panic!() };
        para.lines.iter().filter(|l| l.hyphen.is_some()).count()
    };
    // With markup the deleted soft hyphen is still there, and the word breaks at it.
    assert_eq!(hyphens(false), 1);
    // Final text: the soft hyphen is gone, so is the hyphen.
    assert_eq!(hyphens(true), 0);
}

#[test]
fn deleted_text_adds_no_line_break_opportunity_without_markup() {
    crate::test_fonts();
    // "xx ab\u{ad}cdef" in a column where "xx ab-" and "abcdef" fit but "xx abcdef" does not; only
    // the soft hyphen (5..7) is track-deleted.
    let mut d = Document::from_text("xx ab\u{ad}cdef");
    Arc::make_mut(&mut d.styles).default_chr.font = Some(WRAP_TEST_FONT.into());
    d.format_range(&Pos::body(0, 5), &Pos::body(0, 7), &|c| c.del = Some(0)).unwrap();
    d.format_paragraphs(&Pos::body(0, 0), &Pos::body(0, 0), &|p| p.indent_right = Some(468.0 - 40.0)).unwrap();
    let line_starts = |hide_deleted: bool| {
        let l = layout(&d, &mut LayoutCache::new(), &LayoutOptions { hide_deleted, ..Default::default() });
        let Some(Placed::Lines { para, .. }) = l.pages[0].items.iter().find(|i| matches!(i, Placed::Lines { .. })) else { panic!() };
        para.lines.iter().map(|l| para.clusters.get(l.c0).map_or(usize::MAX, |c| c.start)).collect::<Vec<_>>()
    };
    // With markup the word breaks at the (deleted) soft hyphen.
    assert_eq!(line_starts(false), vec![0, 7]);
    // Final text: "abcdef" is one word, so the line breaks at the space before it.
    assert_eq!(line_starts(true), vec![0, 3]);
}

#[test]
fn hidden_soft_hyphen_is_no_hyphenation_point_when_hidden_text_is_not_shown() {
    crate::test_fonts();
    // As with deletions: "ab\u{ad}cdef" where only the soft hyphen is hidden text.
    let mut d = Document::from_text("ab\u{ad}cdef");
    d.format_range(&Pos::body(0, 2), &Pos::body(0, 4), &|c| c.hidden = Some(true)).unwrap();
    d.format_paragraphs(&Pos::body(0, 0), &Pos::body(0, 0), &|p| p.indent_right = Some(468.0 - 26.0)).unwrap();
    let hyphens = |show_hidden: bool| {
        let l = layout(&d, &mut LayoutCache::new(), &LayoutOptions { show_hidden, ..Default::default() });
        let Some(Placed::Lines { para, .. }) = l.pages[0].items.iter().find(|i| matches!(i, Placed::Lines { .. })) else { panic!() };
        para.lines.iter().filter(|l| l.hyphen.is_some()).count()
    };
    // Hidden text shown: the soft hyphen is there, and the word breaks at it.
    assert_eq!(hyphens(true), 1);
    // Not shown (as printed): no soft hyphen, so no hyphen. Word breaks "abc" / "def" here.
    assert_eq!(hyphens(false), 0);
}

#[test]
fn hidden_text_adds_no_line_break_opportunity_when_not_shown() {
    crate::test_fonts();
    // "xx ab" + hidden soft hyphen or space + "cdef", in a column where "xx ab-" and "abcdef" fit
    // but "xx abcdef" does not. Measured in Word (hidden text not printed): "xx" / "abcdef" both times.
    for hidden in ["\u{ad}", " "] {
        let mut d = Document::from_text(&format!("xx ab{hidden}cdef"));
        Arc::make_mut(&mut d.styles).default_chr.font = Some(WRAP_TEST_FONT.into());
        let end = 5 + hidden.len();
        d.format_range(&Pos::body(0, 5), &Pos::body(0, end), &|c| c.hidden = Some(true)).unwrap();
        d.format_paragraphs(&Pos::body(0, 0), &Pos::body(0, 0), &|p| p.indent_right = Some(468.0 - 40.0)).unwrap();
        let line_starts = |show_hidden: bool| {
            let l = layout(&d, &mut LayoutCache::new(), &LayoutOptions { show_hidden, ..Default::default() });
            let Some(Placed::Lines { para, .. }) = l.pages[0].items.iter().find(|i| matches!(i, Placed::Lines { .. })) else { panic!() };
            para.lines.iter().map(|l| para.clusters.get(l.c0).map_or(usize::MAX, |c| c.start)).collect::<Vec<_>>()
        };
        // Shown, the word breaks at the hidden soft hyphen or space.
        assert_eq!(line_starts(true), vec![0, end], "{hidden:?}");
        // Not shown, "abcdef" is one word, so the line breaks at the space before it.
        assert_eq!(line_starts(false), vec![0, 3], "{hidden:?}");
    }
}

/// How much bigger the second document is in the linear-time tests. At 8x a linear layout costs
/// about 8x as much and a quadratic one about 64x, so a limit of 24x stays far from both.
pub(crate) const SCALE: usize = 8;

/// The CPU time `f` takes on this thread, in seconds. On Linux it is the scheduler's count of the
/// time the thread actually ran, so the time it waits while other work on a busy machine has the
/// CPU is left out. When the counter does not advance, or elsewhere, use wall-clock time.
fn cpu_cost(f: &mut impl FnMut()) -> f64 {
    let ran = || {
        let s = std::fs::read_to_string("/proc/thread-self/schedstat").ok()?;
        s.split_whitespace().next()?.parse::<u64>().ok()
    };
    let (t, before) = (std::time::Instant::now(), ran());
    f();
    match (before, ran()) {
        (Some(a), Some(b)) if b > a => (b - a) as f64 * 1e-9,
        _ => t.elapsed().as_secs_f64(),
    }
}

/// How much more `big` costs than `small`, for `big` an input `scale` times the size of `small`'s:
/// (small, big, big / small), ~`scale` when linear and ~`scale`² when quadratic. Each is the
/// cheapest of several runs after a warm-up, the runs alternating so a busy spell slows both sizes
/// alike; `small` is timed `scale` times over per run, so both timings are long against the clock's
/// resolution.
pub(crate) fn cost_ratio(scale: usize, mut small: impl FnMut(), mut big: impl FnMut()) -> (f64, f64, f64) {
    small();
    big();
    let mut small_scaled = || (0..scale).for_each(|_| small());
    let (mut t1, mut tk) = (f64::INFINITY, f64::INFINITY);
    for _ in 0..5 {
        t1 = t1.min(cpu_cost(&mut small_scaled) / scale as f64);
        tk = tk.min(cpu_cost(&mut big));
    }
    (t1, tk, tk / t1.max(1e-9))
}

/// [`cost_ratio`] of laying out `doc(SCALE * n)` against `doc(n)`.
pub(crate) fn layout_time_ratio(doc: impl Fn(usize) -> Document, n: usize, opts: &LayoutOptions) -> (f64, f64, f64) {
    let (small, big) = (doc(n), doc(SCALE * n));
    cost_ratio(
        SCALE,
        || {
            layout(&small, &mut LayoutCache::new(), opts);
        },
        || {
            layout(&big, &mut LayoutCache::new(), opts);
        },
    )
}

/// A document that is one word with no break opportunity in it, laid out beside a floating
/// object: the word is wider than the span the object leaves, so the measurement that asks
/// whether it would fit the full line runs for every line the word takes.
fn long_word_beside_a_float(n: usize) -> Document {
    use wordcraft_doc::para::{Anchor, Float, Wrap};
    let mut d = Document::from_text(&"a".repeat(n));
    let f = Float { wrap: Wrap::Square, h_rel: Anchor::Column, v_rel: Anchor::Paragraph, dist: 9.0, ..Default::default() };
    d.insert_object(&Pos::body(0, 0), rect_shape(144.0, 100.0, f), &Default::default()).unwrap();
    d
}

/// A word with no break opportunity in it: measuring the word for every line it takes scanned
/// the whole paragraph each time, so the layout was quadratic in the word's length.
#[test]
fn a_long_unbreakable_word_lays_out_in_linear_time() {
    let (t1, tk, ratio) = layout_time_ratio(long_word_beside_a_float, 50_000, &LayoutOptions::default());
    eprintln!("t(n) {t1:.4} s, t(8n) {tk:.4} s, ratio {ratio:.1}");
    assert!(ratio < 24.0, "8x the word took {ratio:.1}x the time ({t1:.4} s → {tk:.4} s): not linear");
}

/// A word a megabyte long lays out in a fraction of a second: it used to take half a minute.
#[test]
fn a_megabyte_long_word_lays_out_quickly() {
    let d = long_word_beside_a_float(1_000_000);
    let t = std::time::Instant::now();
    lay(&d);
    assert!(t.elapsed().as_secs() < 15, "a megabyte-long word took {:?}", t.elapsed());
}

/// A cluster with the given advance and kind, for the tests that measure a word by hand.
fn word_cluster(kind: para::ClKind, adv: f32, break_after: bool) -> para::Cluster {
    para::Cluster { start: 0, end: 1, adv, kind, style: 0, g0: 0, g1: 1, break_after, obj_h: 0.0, obj_d: 0.0, dot: false }
}

/// A word that a negative advance makes narrower still fits beside a float: the width of a word
/// is the sum of its advances, and a font that pulls a glyph back takes the width off again.
#[test]
fn a_word_a_negative_advance_narrows_still_fits() {
    // A hundred points, then a font that pulls the last glyph back by sixty: the word is forty
    // wide, and fits in sixty.
    let clusters = vec![word_cluster(para::ClKind::Text, 100.0, false), word_cluster(para::ClKind::Text, -60.0, true)];
    let widths = para::word_widths(&clusters, &[]);
    assert_eq!(widths[0], 40.0, "the word is the sum of its advances");
    assert!(widths[0] <= 60.0, "a word of forty points fits in sixty");
    assert!(widths[0] > 30.0, "a word of forty points does not fit in thirty");

    // The same word in a document: a font that pulls one glyph back takes the width off the
    // rest of the word too, and the strip a floating object leaves is the row's only span, so
    // the word is measured against it.
    use wordcraft_doc::para::{Anchor, Float, ShapeKind, Wrap};
    // Twelve characters, the sixth of them pulled back by thirty points.
    let word = |pull: f32| {
        let mut d = Document::from_text(&"a".repeat(12));
        if pull != 0.0 {
            d.format_range(&Pos::body(0, 5), &Pos::body(0, 6), &|c| c.spacing = Some(pull)).unwrap();
        }
        d
    };
    fn body(l: &DocLayout) -> &ParaLayout {
        let Some(Placed::Lines { para, .. }) = l.pages[0].items.iter().find(|i| matches!(i, Placed::Lines { story: StoryRef::Body, .. })) else {
            panic!("no body text on the page")
        };
        para
    }
    // The width of the word: the sum of its advances, its last glyph's right edge.
    let width = |d: &Document| {
        let l = lay(d);
        let para = body(&l);
        let ln = &para.lines[0];
        let last = ln.c1 - 1;
        ln.xs[last - ln.c0] + para.clusters[last].adv
    };
    let plain = width(&word(0.0));
    let pulled = width(&word(-30.0));
    assert!(pulled < plain, "a font that pulls a glyph back makes the word narrower: {pulled} against {plain}");
    // The strip the object leaves is half way between the two widths: the word fits it with the
    // glyph pulled back, and does not fit it without.
    let strip = (plain + pulled) / 2.0;
    // The lines of the word laid out beside an object that leaves a strip of that width: (top,
    // left, the clusters it covers, where its text ends, whether it ends inside a word).
    let beside = |d: Document| {
        let float = Float { wrap: Wrap::Square, h_rel: Anchor::Column, v_rel: Anchor::Paragraph, x: 0.0, ..Default::default() };
        let shape = InlineObject::Shape {
            kind: ShapeKind::Rectangle,
            w: 468.0 - strip,
            h: 150.0,
            fill: None,
            stroke: None,
            stroke_width: 1.0,
            float,
            story: None,
            effects: Default::default(),
            freeform: None,
            extra: Default::default(),
        };
        let mut d = d;
        d.insert_object(&Pos::body(0, 0), shape, &Default::default()).unwrap();
        let l = lay(&d);
        let para = body(&l);
        para.lines
            .iter()
            .map(|ln| {
                let inside = ln.c1 > ln.c0 && ln.end == para::LineEnd::Wrap && !para.clusters[ln.c1 - 1].break_after;
                (ln.top, ln.left, ln.c1, ink_end(para, ln), inside)
            })
            .collect::<Vec<_>>()
    };
    // The narrower word fits the strip beside the object and stays there.
    let fits = beside(word(-30.0));
    assert_eq!(fits.len(), 1, "the word fits the strip beside the object: {fits:?}");
    assert!(fits[0].1 >= 468.0 - strip - 0.01, "the word is in the strip beside the object: {fits:?}");
    assert!(fits[0].3 <= 468.0 + 0.01, "the word ends inside the strip: {fits:?}");
    assert!(!fits[0].4, "the word is not split across the strip: {fits:?}");
    // The wider one does not: it moves on whole, below the object, rather than being split
    // across the strip.
    let moves_on = beside(word(0.0));
    assert!(moves_on.iter().any(|l| l.0 >= 150.0 - 0.01 && l.1 < 1.0), "the wider word is laid out below the object: {moves_on:?}");
    assert!(moves_on.iter().all(|l| !l.4), "the word is not split across the strip: {moves_on:?}");
}

/// The words after a break are words of their own: their advances are not counted in the word
/// before them, or a long word followed by a word that pulls the width back would be measured
/// as if the two were one.
#[test]
fn the_words_after_a_break_are_not_counted_in_the_word_before_it() {
    // A word of two ordinary clusters, then a break, then a word that pulls a hundred points
    // back.
    let clusters = vec![
        word_cluster(para::ClKind::Text, 10.0, false),
        word_cluster(para::ClKind::Text, 10.0, true),
        word_cluster(para::ClKind::Text, -100.0, true),
    ];
    let widths = para::word_widths(&clusters, &[]);
    assert_eq!(widths[0], 20.0, "the word after the break must not hold up the word before it");
    assert_eq!(widths[1], 10.0, "the word that ends at the break is a word of its own");
    assert_eq!(widths[2], -100.0);
}

/// A word whose advances alternate in sign is measured by their sum: a font that pulls some
/// glyphs back must not make the measurement walk the word to find out how wide it is.
#[test]
fn a_word_whose_advances_alternate_is_measured_by_its_sum() {
    // A thousand points forward, nine hundred and ninety-nine back: the word is one point wider
    // per pair, so it is far wider than the room even though it is never wider than a thousand
    // points at a time.
    let mut clusters = Vec::new();
    for i in 0..1_000 {
        clusters.push(word_cluster(para::ClKind::Text, if i % 2 == 0 { 1000.0 } else { -999.0 }, false));
    }
    clusters.push(word_cluster(para::ClKind::Text, 0.0, true));
    let widths = para::word_widths(&clusters, &[]);
    assert_eq!(widths[0], 500.0, "the word is the sum of its advances");
}

/// The width of a word is the sum of its advances: the same sum the scan this replaces took, over
/// every shape of word — negative advances, spaces, break opportunities and hyphenation points
/// included.
#[test]
fn a_word_is_measured_by_the_sum_of_its_advances() {
    // The measurement as the scan this replaces took it: the clusters up to the first break
    // opportunity that isn't a hyphenation point, their advance, spaces aside.
    let scan = |clusters: &[para::Cluster], hyph: &[u32], c0: usize| -> f32 {
        let mut done = false;
        let mut word = 0.0f32;
        for (k, c) in (c0..clusters.len()).map(|k| (k, &clusters[k])) {
            let end = c.break_after && hyph.binary_search(&(k as u32)).is_err();
            if done {
                break;
            }
            done = end;
            if c.kind != para::ClKind::Space {
                word += c.adv;
            }
        }
        word
    };
    // A deterministic spread of advances, spaces, breaks and hyphenation points.
    let mut seed = 0x2545_f491_4f6c_dd1du64;
    let mut next = move || {
        seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        (seed >> 33) as i32
    };
    for _ in 0..2_000 {
        let n = 1 + next().unsigned_abs() as usize % 40;
        let clusters: Vec<para::Cluster> = (0..n)
            .map(|_| {
                let kind = if next() % 7 == 0 { para::ClKind::Space } else { para::ClKind::Text };
                word_cluster(kind, (next() % 4000) as f32 / 100.0 - 20.0, next() % 3 == 0)
            })
            .collect();
        let mut hyph = Vec::new();
        if next() % 2 == 0 {
            for _ in 0..next().unsigned_abs() as usize % 4 {
                hyph.push(next().unsigned_abs() % n as u32);
            }
            hyph.sort_unstable();
            hyph.dedup();
        }
        let widths = para::word_widths(&clusters, &hyph);
        for (c0, &width) in widths.iter().enumerate().take(n) {
            // The two sums are the same number, added in a different order: one adds the word's
            // advances from its start and the other from its end, and a float sum depends on
            // the order it is added in. The difference is far below the tenth of a point the
            // comparison that uses the width allows.
            let scan = scan(&clusters, &hyph, c0);
            assert!((width - scan).abs() < 0.001, "the word at cluster {c0} of {clusters:?} is {width}, the scan says {scan}");
        }
    }
}

/// A word with no break opportunity in it whose advances alternate in sign: a run with wide
/// character spacing and a run with condensed spacing, so that half the glyphs are pulled back.
/// The word is wider than the line it is laid out on, so it takes a line per few characters, and
/// measuring it must not walk it again for every one of those lines.
fn long_word_with_alternating_advances_beside_a_float(n: usize) -> Document {
    use wordcraft_doc::para::{Anchor, Float, Wrap};
    let mut d = Document::from_text(&"a".repeat(n));
    // One run per block of characters, alternating the spacing: half the glyphs are pulled back.
    let block = 64;
    let p = d.para_mut(StoryRef::Body, &Path::top(0)).unwrap();
    p.runs = (0..n.div_ceil(block))
        .map(|i| wordcraft_doc::Run {
            len: block.min(n - i * block),
            props: wordcraft_doc::CharProps { spacing: Some(if i % 2 == 0 { 50.0 } else { -50.0 }), ..Default::default() },
        })
        .collect();
    let f = Float { wrap: Wrap::Square, h_rel: Anchor::Column, v_rel: Anchor::Paragraph, dist: 9.0, ..Default::default() };
    d.insert_object(&Pos::body(0, 0), rect_shape(144.0, 100.0, f), &Default::default()).unwrap();
    d
}

/// A word whose advances alternate in sign lays out in linear time: the measurement that asks
/// whether it would fit the full line must not walk the word again for every line it takes.
#[test]
fn a_word_with_alternating_advances_lays_out_in_linear_time() {
    let (t1, tk, ratio) = layout_time_ratio(long_word_with_alternating_advances_beside_a_float, 50_000, &LayoutOptions::default());
    eprintln!("t(n) {t1:.4} s, t(8n) {tk:.4} s, ratio {ratio:.1}");
    assert!(ratio < 24.0, "8x the word took {ratio:.1}x the time ({t1:.4} s → {tk:.4} s): not linear");
}

#[test]
fn hidden_separators_with_automatic_hyphenation_lay_out_in_linear_time() {
    crate::test_fonts();
    // "hyphenation " n times with every space hidden: one joined word of 11n letters reaches the
    // hyphenator, whose limit checks once rescanned the word for every candidate point.
    let doc = |n: usize| {
        let mut d = Document::from_text(&"hyphenation ".repeat(n));
        d.settings.auto_hyphenation = true;
        for k in 0..n {
            let at = 12 * k + 11;
            d.format_range(&Pos::body(0, at), &Pos::body(0, at + 1), &|c| c.hidden = Some(true)).unwrap();
        }
        d
    };
    let (t1, tk, ratio) = layout_time_ratio(doc, 800, &LayoutOptions::default());
    eprintln!("t(n) {t1:.4} s, t(8n) {tk:.4} s, ratio {ratio:.1}");
    assert!(ratio < 24.0, "8x the text took {ratio:.1}x the time ({t1:.4} s → {tk:.4} s): not linear");
}

/// n one-line paragraphs, each starting a page.
fn page_break_paragraphs(n: usize) -> Document {
    let mut d = Document::from_text(&"hello\n".repeat(n));
    for i in 0..n {
        d.para_mut(StoryRef::Body, &Path::top(i)).unwrap().props.page_break_before = Some(true);
    }
    d
}

/// A document of many paragraphs lays out in linear time. Every page asks the layout for the
/// section properties (for the wrap areas of the headers and footers), and that used to rebuild
/// the section list by walking the whole body, so a document of n paragraphs paid n body-length
/// scans: a two-megabyte file took half a minute before the first page was drawn.
#[test]
fn a_document_of_many_paragraphs_lays_out_in_linear_time() {
    let (t1, tk, ratio) = layout_time_ratio(page_break_paragraphs, 2_500, &LayoutOptions::default());
    eprintln!("t(n) {t1:.4} s, t(8n) {tk:.4} s, ratio {ratio:.1}");
    assert!(ratio < 24.0, "8x the paragraphs took {ratio:.1}x the time ({t1:.4} s → {tk:.4} s): not linear");
}

#[test]
fn many_hidden_float_anchors_lay_out_in_linear_time() {
    crate::test_fonts();
    // n square-wrapped shapes, each anchored in its own hidden run (bold alternating, so runs do not
    // merge), markup on: finding each anchor's formatting once rescanned the runs from the start.
    let doc = |n: usize| {
        let float = wordcraft_doc::para::Float {
            wrap: wordcraft_doc::para::Wrap::Square,
            h_rel: wordcraft_doc::para::Anchor::Column,
            v_rel: wordcraft_doc::para::Anchor::Paragraph,
            x: 0.0,
            y: 0.0,
            dist: 9.0,
            dist_top: 9.0,
            dist_bottom: 9.0,
            ..Default::default()
        };
        let shape = InlineObject::Shape {
            kind: wordcraft_doc::para::ShapeKind::Rectangle,
            w: 20.0,
            h: 20.0,
            fill: None,
            stroke: None,
            stroke_width: 1.0,
            float,
            story: None,
            freeform: None,
            effects: Default::default(),
            extra: Default::default(),
        };
        let obj = wordcraft_doc::para::OBJ.to_string();
        let mut p = wordcraft_doc::Paragraph::with_text("Text ", Default::default());
        p.text.push_str(&obj.repeat(n));
        p.objects = vec![shape; n];
        p.runs = std::iter::once(wordcraft_doc::Run { len: 5, props: Default::default() })
            .chain((0..n).map(|i| wordcraft_doc::Run {
                len: obj.len(),
                props: wordcraft_doc::CharProps { hidden: Some(true), bold: Some(i % 2 == 0), ..Default::default() },
            }))
            .collect();
        let mut d = Document::from_text("");
        d.body = vec![wordcraft_doc::para_block(p)];
        d
    };
    // Correct first: hidden anchors place no shape, shown ones do.
    let shapes = |show_hidden: bool| {
        let l = layout(&doc(50), &mut LayoutCache::new(), &LayoutOptions { show_hidden, ..Default::default() });
        l.pages.iter().flat_map(|p| p.items.iter()).filter(|i| matches!(i, Placed::Shape { .. })).count()
    };
    assert_eq!((shapes(false), shapes(true)), (0, 50));
    let (t1, tk, ratio) = layout_time_ratio(doc, 10_000, &LayoutOptions::default());
    eprintln!("t(n) {t1:.4} s, t(8n) {tk:.4} s, ratio {ratio:.1}");
    assert!(ratio < 24.0, "8x the anchors took {ratio:.1}x the time ({t1:.4} s → {tk:.4} s): not linear");
}

#[test]
fn fragmented_hidden_text_lays_out_like_the_visible_text() {
    crate::test_fonts();
    // Visible words with a hidden multi-byte char after every visible one, formatted in alternating
    // runs (bold on and off) so no two left-out runs are adjacent or merge. The plain paragraph has
    // the same visible runs without the hidden chars, so both lay out the same widths whatever font
    // the default font resolves to.
    let words = "Hyphenation wraps international words across narrow columns of text ".repeat(6);
    let mut d = Document::from_text("");
    d.settings.auto_hyphenation = true;
    let mut plain = Document::from_text("");
    plain.settings.auto_hyphenation = true;
    let mut text = String::new();
    let mut kept = Vec::new();
    for (i, c) in words.chars().enumerate() {
        kept.push(text.len());
        text.push(c);
        let at = text.len();
        text.push('é');
        let n = d.para(StoryRef::Body, &Path::top(0)).unwrap().len();
        let bold = …162672 tokens truncated…acts[0].levels[0];
    level.indent = 100.0;
    level.hanging = 0.0;
    let props = ParaProps {
        space_before: Some(0.0),
        space_after: Some(0.0),
        line_spacing: Some(LineSpacing::Exactly(12.0)),
        widow_control: Some(true),
        ..Default::default()
    };
    let para = |text: &str, props: ParaProps| {
        let mut p =
            Paragraph::with_text(text, wordcraft_doc::CharProps { font: Some("Source Sans 3".into()), size: Some(12.0), ..Default::default() });
        p.props = props;
        wordcraft_doc::para_block(p)
    };
    doc.body = vec![
        para("Filler", ParaProps { line_spacing: Some(LineSpacing::Exactly(136.0)), ..props.clone() }),
        para("Heading", ParaProps { keep_next: Some(true), keep_lines: Some(true), ..props.clone() }),
        para("Alpha beta gamma delta", ParaProps { numbering: Some(NumRef { num: id, level: 0 }), ..props }),
    ];
    let layout = lay(&doc);
    assert_eq!(layout.caret(&Pos::body(2, 0)).unwrap().page, 1, "the indented list cannot start below the heading");
    assert_eq!(layout.caret(&Pos::body(1, 0)).unwrap().page, 1, "the kept heading must follow it");
    let labels: Vec<_> = layout
        .pages
        .iter()
        .flat_map(|p| &p.items)
        .filter_map(|item| match item {
            Placed::Lines { para, .. } => para.label.as_ref().map(|label| label.text.as_str()),
            _ => None,
        })
        .collect();
    assert_eq!(labels, ["1."], "lookahead must not consume a list counter");
}

/// #2712: Word breaks a line right after a hyphen followed by a digit ("2016-" / "2022"), where
/// UAX #14 keeps the hyphen and the number together. Sweeping the text width, the first width
/// that wraps `Deed 2016-2022` must break after the hyphen -- not move the range whole (the
/// break lands at the space) and not split it inside a number.
#[test]
fn a_hyphen_before_digits_is_a_break_opportunity_like_word() {
    for m in 100..=540 {
        let margin_right = m as f32;
        let text = "Deed 2016-2022";
        let mut d = Document::from_text(text);
        d.last_section.margin_right = margin_right;
        let l = lay(&d);
        let line_texts = |l: &DocLayout| -> Vec<String> {
            l.pages[0]
                .items
                .iter()
                .filter_map(|it| {
                    if let Placed::Lines { para, l0, l1, .. } = it {
                        Some(para.lines[*l0..*l1].iter().map(|line| text[line.start..line.stop].to_string()).collect::<Vec<_>>())
                    } else {
                        None
                    }
                })
                .flatten()
                .collect()
        };
        let lines = line_texts(&l);
        if lines.len() < 2 {
            continue; // still fits on one line at this width
        }
        assert_eq!(lines[0], "Deed 2016-", "the first wrap breaks after the hyphen: {lines:?}");
        assert_eq!(lines[1], "2022", "the range continues after it: {lines:?}");
        return;
    }
    panic!("no width in the sweep wrapped the paragraph");
}

/// #2712 scopes the new break to U+002D: a non-breaking hyphen (U+2011) and a minus sign
/// (U+2212) gain no break, so the first wrap moves the range whole, breaking at the space.
#[test]
fn a_non_breaking_hyphen_and_a_minus_sign_stay_unbreakable() {
    for (token, hyphen) in [("2016\u{2011}2022", '\u{2011}'), ("2016\u{2212}2022", '\u{2212}')] {
        let text = format!("Deed {token}");
        for m in 100..=540 {
            let mut d = Document::from_text(&text);
            d.last_section.margin_right = m as f32;
            let l = lay(&d);
            let lines: Vec<String> = l.pages[0]
                .items
                .iter()
                .filter_map(|it| {
                    if let Placed::Lines { para, l0, l1, .. } = it {
                        Some(para.lines[*l0..*l1].iter().map(|line| text[line.start..line.stop].to_string()).collect::<Vec<_>>())
                    } else {
                        None
                    }
                })
                .flatten()
                .collect();
            if lines.len() < 2 {
                continue; // still fits on one line at this width
            }
            // The first wrap breaks at the space: the protected hyphen is not a break. (Narrower
            // widths fall into the break-anywhere fallback that splits too-long words between
            // characters, which the sweep leaves out, as the issue does.)
            assert_eq!(lines[0], "Deed ", "the only break is at the space, not after {hyphen}: {lines:?}");
            assert_eq!(lines[1], token, "the range stays whole: {lines:?}");
            break;
        }
    }
}

/// #3408: Word breaks a compound right after a hyphen that a slash follows ("Dataprotection-" /
/// "/Secrecyviolation"), where UAX #14 forbids a break before the slash. The first width that
/// wraps must break after the hyphen, not move the compound whole, and the slash stays with its
/// word. Table auto-fit reads the same break, so such a column no longer widens to the compound.
/// Word keeps "fitness-/1589" whole: a slash before a digit gains no break.
#[test]
fn a_hyphen_before_a_slash_is_a_break_opportunity_like_word() {
    let digits = "Deed fitness-/1589";
    let mut wrapped = false;
    for m in 100..=540 {
        let mut d = Document::from_text(digits);
        d.last_section.margin_right = m as f32;
        let l = lay(&d);
        let lines: Vec<&str> = l.pages[0]
            .items
            .iter()
            .filter_map(|it| if let Placed::Lines { para, l0, l1, .. } = it { Some(&para.lines[*l0..*l1]) } else { None })
            .flatten()
            .map(|line| &digits[line.start..line.stop])
            .collect();
        if lines.len() >= 2 {
            assert_eq!(lines, ["Deed ", "fitness-/1589"], "the first wrap breaks at the space, not after the hyphen");
            wrapped = true;
            break;
        }
    }
    assert!(wrapped, "no width in the sweep wrapped the digit case");

    let text = "Deed Dataprotection-/Secrecyviolation";
    for m in 100..=540 {
        let mut d = Document::from_text(text);
        d.last_section.margin_right = m as f32;
        let l = lay(&d);
        let lines: Vec<String> = l.pages[0]
            .items
            .iter()
            .filter_map(|it| {
                if let Placed::Lines { para, l0, l1, .. } = it {
                    Some(para.lines[*l0..*l1].iter().map(|line| text[line.start..line.stop].to_string()).collect::<Vec<_>>())
                } else {
                    None
                }
            })
            .flatten()
            .collect();
        if lines.len() < 2 {
            continue; // still fits on one line at this width
        }
        assert_eq!(lines[0], "Deed Dataprotection-", "the first wrap breaks after the hyphen: {lines:?}");
        assert_eq!(lines[1], "/Secrecyviolation", "the slash starts the next line with its word: {lines:?}");
        return;
    }
    panic!("no width in the sweep wrapped the paragraph");
}

/// docxide-pdf `dot-open-token*`: UAX #14 allows a break between a full stop and an opening
/// parenthesis, Word doesn't. The first width that wraps `Deed 2023.(Example` breaks at the space.
#[test]
fn a_full_stop_before_a_parenthesis_is_no_break() {
    let text = "Deed 2023.(Example";
    for m in 100..=540 {
        let mut d = Document::from_text(text);
        d.last_section.margin_right = m as f32;
        let l = lay(&d);
        let lines: Vec<String> = l.pages[0]
            .items
            .iter()
            .filter_map(|it| {
                if let Placed::Lines { para, l0, l1, .. } = it {
                    Some(para.lines[*l0..*l1].iter().map(|line| text[line.start..line.stop].to_string()).collect::<Vec<_>>())
                } else {
                    None
                }
            })
            .flatten()
            .collect();
        if lines.len() < 2 {
            continue; // still fits on one line at this width
        }
        assert_eq!(lines, ["Deed ", "2023.(Example"], "the first wrap breaks at the space");
        return;
    }
    panic!("no width in the sweep wrapped the paragraph");
}

#[test]
fn layout_cache_invalidates_on_changed_filename() {
    let mut doc = Document::new();
    let mut p = Paragraph::new();
    p.insert_object(
        0,
        InlineObject::Field { instr: "FILENAME".into(), result: String::new(), locked: false, dirty: false, code: false, form: String::new() },
        &Default::default(),
    )
    .unwrap();
    doc.body = vec![wordcraft_doc::para_block(p)];

    let shown = |l: &DocLayout| -> String {
        l.pages[0]
            .items
            .iter()
            .find_map(|it| if let Placed::Lines { para, .. } = it { para.shown.first().map(|s| s.1.clone()) } else { None })
            .unwrap_or_default()
    };

    let mut cache = LayoutCache::new();
    let opts_a = LayoutOptions { filename: Some(Arc::from("a.docx")), filepath: Some(Arc::from("/docs/a.docx")), ..Default::default() };
    let l_a = layout(&doc, &mut cache, &opts_a);
    let text_a = shown(&l_a);
    assert_eq!(text_a, "a.docx", "first layout shows a.docx");

    let opts_b = LayoutOptions { filename: Some(Arc::from("b.docx")), filepath: Some(Arc::from("/docs/b.docx")), ..Default::default() };
    let l_b = layout(&doc, &mut cache, &opts_b);
    let text_b = shown(&l_b);
    assert_eq!(text_b, "b.docx", "second layout with same cache shows b.docx");
}

#[test]
fn layout_cache_invalidates_on_changed_filepath() {
    let mut doc = Document::new();
    let mut p = Paragraph::new();
    p.insert_object(
        0,
        InlineObject::Field { instr: r"FILENAME \p".into(), result: String::new(), locked: false, dirty: false, code: false, form: String::new() },
        &Default::default(),
    )
    .unwrap();
    doc.body = vec![wordcraft_doc::para_block(p)];

    let shown = |l: &DocLayout| -> String {
        l.pages[0]
            .items
            .iter()
            .find_map(|it| if let Placed::Lines { para, .. } = it { para.shown.first().map(|s| s.1.clone()) } else { None })
            .unwrap_or_default()
    };

    let mut cache = LayoutCache::new();
    let opts_a = LayoutOptions { filename: Some(Arc::from("a.docx")), filepath: Some(Arc::from("/path/one/a.docx")), ..Default::default() };
    let l_a = layout(&doc, &mut cache, &opts_a);
    let text_a = shown(&l_a);
    assert_eq!(text_a, "/path/one/a.docx", "first layout shows /path/one/a.docx");

    let opts_b = LayoutOptions { filename: Some(Arc::from("a.docx")), filepath: Some(Arc::from("/path/two/a.docx")), ..Default::default() };
    let l_b = layout(&doc, &mut cache, &opts_b);
    let text_b = shown(&l_b);
    assert_eq!(text_b, "/path/two/a.docx", "second layout with same cache shows /path/two/a.docx");
}

/// Keep with next takes the same 0.01 pt of slack as the other fit checks: a kept paragraph and
/// the next one's first line that fill the page to within it stay on the page.
#[test]
fn keep_with_next_fits_to_the_hundredth_of_a_point() {
    use wordcraft_doc::props::LineSpacing;
    let exact =
        |h: f32| ParaProps { space_before: Some(0.0), space_after: Some(0.0), line_spacing: Some(LineSpacing::Exactly(h)), ..Default::default() };
    let mut d = Document::from_text("Filler\nKept\nNext");
    let body = d.last_section.page_h - d.last_section.margin_top - d.last_section.margin_bottom;
    d.para_mut(StoryRef::Body, &Path::top(0)).unwrap().props = exact(body - 24.0 + 0.005);
    d.para_mut(StoryRef::Body, &Path::top(1)).unwrap().props = ParaProps { keep_next: Some(true), ..exact(12.0) };
    d.para_mut(StoryRef::Body, &Path::top(2)).unwrap().props = exact(12.0);
    let l = lay(&d);
    assert_eq!(l.caret(&Pos::body(1, 0)).unwrap().page, 0, "the kept paragraph stays on the first page");
    assert_eq!(l.caret(&Pos::body(2, 0)).unwrap().page, 0, "and so does the next one's first line");
}

/// A table of `rows` one-line rows with `border`-wide borders after `fill` filler lines.
fn ruled_table(fill: usize, rows: usize, border: f32) -> Document {
    let mut d = Document::from_text("filler\n".repeat(fill).trim_end());
    let mut t = Table::new(rows, 1, 468.0);
    t.props.style = None;
    t.props.borders = Some(Borders::all(Border::single(border)));
    for row in &mut t.rows {
        row.cells[0].blocks = vec![wordcraft_doc::para_block(wordcraft_doc::Paragraph::with_text("row", Default::default()))];
    }
    d.insert_block(StoryRef::Body, &Path::top(fill), wordcraft_doc::Block::Table(t)).unwrap();
    d
}

/// A row that another row follows needs the border below it on the page too, as in Word: the
/// last row on a page ends at least that border's width above the bottom margin (a row ending
/// 0.05 pt short of the margin with a 0.5 pt border under it moves on). Swept over the filler
/// lines above the table, so the row nearest the margin lands at every offset from it.
#[test]
fn a_row_leaves_room_for_the_border_below_it() {
    for border in [0.5f32, 3.0] {
        for fill in 20..40 {
            let d = ruled_table(fill, 80, border);
            let l = lay(&d);
            let bottom = d.last_section.page_h - d.last_section.margin_bottom;
            let end = l.pages[0]
                .items
                .iter()
                .filter_map(|it| if let Placed::Cell { rect, .. } = it { Some(rect.bottom()) } else { None })
                .fold(0.0f32, f32::max);
            assert!(end + border <= bottom + 0.02, "border {border}, {fill} filler lines: the last row ends at {end}, the margin at {bottom}");
        }
    }
}

/// A horizontal rule (VML `o:hr`) takes its whole line and is drawn across its share of it,
/// placed by its alignment (Word, measured: 50 % centred, left, right). In a table cell it
/// follows its column instead of widening the table.
#[test]
fn horizontal_rules_take_their_line() {
    use wordcraft_doc::connector::HorizontalRule;
    use wordcraft_doc::props::Align;
    let rule = |pct: f32, align: Align| {
        let mut o = rect_shape(0.0, 1.5, Default::default());
        if let InlineObject::Shape { extra, .. } = &mut o {
            extra.hr = Some(HorizontalRule { pct, align, standard: true });
        }
        o
    };
    let drawn = |d: &Document| -> Vec<wordcraft_geom::Rect> {
        let l = lay(d);
        display::page_display(d, &l.pages[0], &display::DisplayOptions::default())
            .iter()
            .filter_map(
                |i| if let display::Draw::Shape { rect, kind: wordcraft_doc::para::ShapeKind::Rectangle, .. } = i { Some(*rect) } else { None },
            )
            .collect()
    };
    let mut d = Document::from_text("a\nb");
    d.insert_object(&Pos::body(1, 0), rule(50.0, Align::Center), &Default::default()).unwrap();
    d.insert_object(&Pos::body(0, 0), rule(100.0, Align::Left), &Default::default()).unwrap();
    let s = drawn(&d);
    let (full, half) = (s[0], s[1]);
    assert!(full.w > 400.0, "the whole line: {full:?}");
    assert!((half.w - full.w / 2.0).abs() < 0.01 && (half.x - (full.x + full.w / 4.0)).abs() < 0.01, "{half:?} in {full:?}");
    let mut cell = Table::new(1, 1, 200.0);
    let mut p = Paragraph::default();
    p.insert_object(0, rule(100.0, Align::Left), &Default::default()).unwrap();
    cell.rows[0].cells[0].blocks = vec![wordcraft_doc::para_block(p)];
    d.body = vec![Arc::new(Block::Table(cell))];
    let s = drawn(&d);
    assert!(s.first().is_some_and(|r| r.w > 150.0 && r.w < 200.0), "the cell's width, not the page's: {s:?}");
    // Word's numbers (A4, 1 inch margins, a 100 pt float): beside a float a rule keeps its width
    // and is placed in what the line has left, a first-line indent moving the end of that room
    // along, so a whole-line rule runs past the margin as in Word.
    let beside = |pct: f32, align: Align, side: Option<f32>, first: f32| {
        use wordcraft_doc::para::{Anchor, Float, Wrap};
        let mut d = Document::from_text("a");
        if first != 0.0 {
            let mut p = d.body[0].as_para().unwrap().clone();
            p.props.indent_first = Some(first);
            d.body[0] = Arc::new(Block::Para(p));
        }
        d.insert_object(&Pos::body(0, 0), rule(pct, align), &Default::default()).unwrap();
        if let Some(x) = side {
            let f = Float { wrap: Wrap::Square, h_rel: Anchor::Column, v_rel: Anchor::Paragraph, x, dist: 9.0, ..Default::default() };
            d.insert_object(&Pos::body(0, 0), rect_shape(100.0, 100.0, f), &Default::default()).unwrap();
        }
        let l = lay(&d);
        display::page_display(&d, &l.pages[0], &display::DisplayOptions::default())
            .iter()
            .find_map(|i| match i {
                display::Draw::Shape { rect, kind: wordcraft_doc::para::ShapeKind::Rectangle, .. } if rect.h < 10.0 => {
                    Some((rect.x, rect.x + rect.w))
                }
                _ => None,
            })
            .unwrap()
    };
    let (x0, x1) = beside(100.0, Align::Left, None, 0.0);
    let (tw, half) = (x1 - x0, (x1 - x0) / 2.0);
    let near =
        |got: (f32, f32), want: (f32, f32)| assert!((got.0 - want.0).abs() < 0.5 && (got.1 - want.1).abs() < 0.5, "{got:?}, Word's rule {want:?}");
    // A float on the left: the room is from past it to the margin.
    let lo = x0 + 109.0;
    near(beside(50.0, Align::Left, Some(0.0), 0.0), (lo, lo + half));
    near(beside(50.0, Align::Center, Some(0.0), 0.0), ((lo + x1 - half) / 2.0, (lo + x1 + half) / 2.0));
    near(beside(50.0, Align::Right, Some(0.0), 0.0), (x1 - half, x1));
    near(beside(100.0, Align::Left, Some(0.0), 0.0), (lo, lo + tw));
    // A float on the right: from the margin to before it.
    let hi = x1 - 109.0;
    near(beside(50.0, Align::Center, Some(tw - 100.0), 0.0), ((x0 + hi - half) / 2.0, (x0 + hi + half) / 2.0));
    near(beside(50.0, Align::Right, Some(tw - 100.0), 0.0), (hi - half, hi));
    near(beside(100.0, Align::Left, Some(tw - 100.0), 0.0), (x0, x1));
    // A first-line indent moves the room along.
    near(beside(100.0, Align::Left, None, 36.0), (x0 + 36.0, x1 + 36.0));
    near(beside(50.0, Align::Center, None, 36.0), (x0 + 36.0 + half / 2.0, x1 + 36.0 - half / 2.0));
    near(beside(50.0, Align::Right, None, 36.0), (x1 + 36.0 - half, x1 + 36.0));
    // Measuring a table column: a rule of its own width asks for it, a share of the line for none.
    let measured = |pct: f32| {
        let mut o = rect_shape(150.0, 1.5, Default::default());
        if let InlineObject::Shape { extra, .. } = &mut o {
            extra.hr = Some(HorizontalRule { pct, align: Align::Left, standard: false });
        }
        let mut t = Table::new(1, 1, 200.0);
        let mut p = Paragraph::default();
        p.insert_object(0, o, &Default::default()).unwrap();
        t.rows[0].cells[0].blocks = vec![wordcraft_doc::para_block(p)];
        crate::autofit_widths(&crate::measure_table_columns(&d, &t), 451.0)[0]
    };
    assert!(measured(0.0) >= 150.0, "{}", measured(0.0));
    assert!(measured(100.0) < 150.0, "{}", measured(100.0));
}

/// A line's height with pictures raised or lowered and with legacy check boxes, in Word's numbers
/// (Arial at 10 and 20 pt, single, 1.15 and 1.5 spacing).
#[test]
fn lowered_pictures_and_check_boxes_take_words_line_heights() {
    use wordcraft_doc::props::LineSpacing;
    // A paragraph at `m` spacing from a lead, a picture and how far its run is raised: the line's
    // height, its picture's top under the line's, and the font's height.
    let line_of = |lead: &str, m: f32, h: f32, raise: f32| {
        let mut d = Document::from_text(lead);
        let props = wordcraft_doc::CharProps { position: Some(raise), ..Default::default() };
        let p = d.para_mut(StoryRef::Body, &Path::top(0)).unwrap();
        p.props.line_spacing = Some(LineSpacing::Multiple(m));
        let at = p.len();
        p.insert_object(at, picture(40.0, h, Default::default()), &props).unwrap();
        let l = lay(&d);
        let Some(Placed::Lines { para, .. }) = l.pages[0].items.iter().find(|i| matches!(i, Placed::Lines { .. })) else { panic!() };
        let c = para.clusters.iter().find(|c| matches!(c.kind, para::ClKind::Object(_))).unwrap();
        let (line, st) = (&para.lines[0], &para.styles[c.style as usize]);
        (line.height, line.baseline - c.obj_h.max(0.0) - line.top, st.ascent + st.descent)
    };
    for m in [1.0, 1.15, 1.5] {
        let share = |font: f32| (m - 1.0) * font;
        // Alone, lowered or not: the picture's height and the spacing's share, its top the line's.
        for lowered in [0.0, 10.0, 30.0] {
            let (h, top, font) = line_of("", m, 30.0, -lowered);
            assert!((h - 30.0 - share(font)).abs() < 0.01 && top.abs() < 0.01, "lowered {lowered} at {m}: {h}, {top}");
        }
        // Raised: as far as it reaches.
        let (h, top, font) = line_of("", m, 30.0, 10.0);
        assert!((h - 40.0 - share(font)).abs() < 0.01 && top.abs() < 0.01, "raised at {m}: {h}, {top}");
        // Shorter than the font: the font's height, and it ends at the font's bottom.
        for (raise, reach) in [(0.0, 6.0), (-3.0, 6.0), (3.0, 9.0)] {
            let (h, top, font) = line_of("", m, 6.0, raise);
            assert!((h - font * m).abs() < 0.01 && (top - (font - reach)).abs() < 0.01, "{raise} at {m}: {h}, {top}");
        }
        // Lowered further than its height: as far down as it's lowered, the font's height at least,
        // its top on the baseline (Word: a 6 pt picture lowered 10 pt takes the font's 11.5 pt,
        // lowered 30 pt 30 pt; beside text, the text's ascent and the 30 pt).
        let (h, top, font) = line_of("", m, 6.0, -10.0);
        assert!((h - font * m).abs() < 0.01 && (top - (font - 10.0)).abs() < 0.01, "6 lowered 10 at {m}: {h}, {top}");
        let (h, top, font) = line_of("", m, 6.0, -30.0);
        assert!((h - 30.0 - share(font)).abs() < 0.01 && top.abs() < 0.01, "6 lowered 30 at {m}: {h}, {top}");
        let (h, top, font) = line_of("x", m, 6.0, -30.0);
        let asc = h - 30.0 - share(font);
        assert!(asc > 0.0 && asc < font && (top - asc).abs() < 0.01, "6 lowered 30 beside text at {m}: {h}, {top}");
        // Beside text, lowered: the text's ascent is no matter, the picture reaching further.
        let (h, top, font) = line_of("x", m, 30.0, -10.0);
        assert!((h - 30.0 - share(font)).abs() < 0.01 && top.abs() < 0.01, "beside text at {m}: {h}, {top}");
    }
    // Exact spacing keeps the line's box and its baseline: a picture lowered past its height sits
    // where one lowered by its height does.
    let exact = |raise: f32| {
        let mut d = Document::from_text("x");
        let props = wordcraft_doc::CharProps { position: Some(raise), ..Default::default() };
        let p = d.para_mut(StoryRef::Body, &Path::top(0)).unwrap();
        p.props.line_spacing = Some(LineSpacing::Exactly(20.0));
        let at = p.len();
        p.insert_object(at, picture(40.0, 6.0, Default::default()), &props).unwrap();
        let l = lay(&d);
        let Some(Placed::Lines { para, .. }) = l.pages[0].items.iter().find(|i| matches!(i, Placed::Lines { .. })) else { panic!() };
        (para.lines[0].height, para.lines[0].baseline - para.lines[0].top)
    };
    let (h, base) = exact(-6.0);
    assert!((h - 20.0).abs() < 0.01, "{h}");
    assert_eq!(exact(-30.0), (h, base));
    // A check box sizes its line as text of its size, alone or beside text.
    let box_line = |lead: &str, m: f32, size: u32| {
        let mut d = Document::from_text(lead);
        let form = format!(r#"<w:ffData><w:name w:val="Box"/><w:checkBox><w:size w:val="{size}"/></w:checkBox></w:ffData>"#);
        let field = InlineObject::Field { instr: "FORMCHECKBOX".into(), result: String::new(), locked: false, dirty: false, code: false, form };
        let props = wordcraft_doc::CharProps { size: Some(10.0), ..Default::default() };
        let p = d.para_mut(StoryRef::Body, &Path::top(0)).unwrap();
        p.props.line_spacing = Some(LineSpacing::Multiple(m));
        let at = p.len();
        p.insert_object(at, field, &props).unwrap();
        let l = lay(&d);
        let Some(Placed::Lines { para, .. }) = l.pages[0].items.iter().find(|i| matches!(i, Placed::Lines { .. })) else { panic!() };
        let c = para.clusters.iter().find(|c| matches!(c.kind, para::ClKind::Object(_))).unwrap();
        let st = &para.styles[c.style as usize];
        (para.lines[0].height, st.ascent + st.descent)
    };
    for m in [1.0, 1.5] {
        let (h, font) = box_line("", m, 40);
        assert!((h - font * 2.0 * m).abs() < 0.01, "a 20 pt box in 10 pt text at {m}: {h}, font {font}");
    }
}

/// A line of only a picture (and spaces) ends at the picture's bottom; with text beside the
/// picture the text's descent goes below it too (Word, measured with Aptos, Arial and Times New
/// Roman at 10 and 20 pt: a 30 pt picture alone or after spaces takes 30 pt at single spacing,
/// after a word 30 pt plus the descent).
#[test]
fn a_line_of_only_pictures_takes_no_descent() {
    use wordcraft_doc::props::LineSpacing;
    let line_of = |lead: &str| {
        let mut d = Document::from_text(lead);
        if let Some(wordcraft_doc::Block::Para(p)) = d.body.get_mut(0).map(std::sync::Arc::make_mut) {
            p.props.line_spacing = Some(LineSpacing::Multiple(1.0));
            let at = p.len();
            p.insert_object(at, picture(60.0, 30.0, Default::default()), &Default::default()).unwrap();
        }
        let l = lay(&d);
        let Some(Placed::Lines { para, .. }) = l.pages[0].items.iter().find(|i| matches!(i, Placed::Lines { .. })) else { panic!() };
        (para.lines[0].height, para.lines[0].baseline - para.lines[0].top, para.styles[0].descent)
    };
    for lead in ["", "    "] {
        let (h, base, _) = line_of(lead);
        assert!((h - 30.0).abs() < 0.01 && (base - 30.0).abs() < 0.01, "{lead:?}: line {h}, baseline {base}");
    }
    let (h, base, desc) = line_of("Text ");
    assert!((h - (30.0 + desc)).abs() < 0.01 && (base - 30.0).abs() < 0.01, "with text: line {h}, baseline {base}");
    // A line break ending the picture's line is no text.
    let mut d = Document::from_text("");
    if let Some(wordcraft_doc::Block::Para(p)) = d.body.get_mut(0).map(std::sync::Arc::make_mut) {
        p.props.line_spacing = Some(LineSpacing::Multiple(1.0));
        p.insert_object(0, picture(60.0, 30.0, Default::default()), &Default::default()).unwrap();
        let at = p.len();
        p.insert_text(at, &format!("{}Next", wordcraft_doc::para::LINE_BREAK), &Default::default()).unwrap();
    }
    let l = lay(&d);
    let Some(Placed::Lines { para, .. }) = l.pages[0].items.iter().find(|i| matches!(i, Placed::Lines { .. })) else { panic!() };
    assert!(para.lines.len() == 2 && (para.lines[0].height - 30.0).abs() < 0.01, "picture, then a line break: {}", para.lines[0].height);
}

/// A kept-together paragraph starting a continuous section low on a page, with room there for
/// one of its lines, moves whole to the next page, where it fits (#3907 review).
#[test]
fn kept_lines_at_a_low_continuous_section_move_to_the_next_page() {
    use wordcraft_doc::props::LineSpacing;
    use wordcraft_doc::section::{SectionProps, SectionStart};
    let fill = 53; // 53 lines of 12 pt leave one line of a 648 pt body
    let text = format!("{}{}", "x\n".repeat(fill), "word ".repeat(40));
    let mut d = Document::from_text(&text);
    for i in 0..=fill {
        let p = d.para_mut(StoryRef::Body, &Path::top(i)).unwrap();
        p.props.line_spacing = Some(LineSpacing::Exactly(12.0));
        p.props.space_before = Some(0.0);
        p.props.space_after = Some(0.0);
    }
    d.para_mut(StoryRef::Body, &Path::top(fill - 1)).unwrap().section =
        Some(Box::new(SectionProps { start: SectionStart::Continuous, ..Default::default() }));
    d.last_section.start = SectionStart::Continuous;
    let kept = d.para_mut(StoryRef::Body, &Path::top(fill)).unwrap();
    kept.props.keep_lines = Some(true);
    kept.props.widow_control = Some(false);
    let l = lay(&d);
    let last_x = l.caret(&Pos::body(fill - 1, 0)).unwrap();
    let first = l.caret(&Pos::body(fill, 0)).unwrap();
    assert_eq!(last_x.page, 0, "the filler fits on page 1: {last_x:?}");
    assert_eq!(first.page, 1, "kept paragraph moves whole: {first:?}");
}

/// A kept-together paragraph in a continuous section is measured against the page it would move
/// to, not the one it starts on: below a tall different first-page header it doesn't fit page 1's
/// body but does page 2's, so it moves there whole (#3907 CodeRabbit).
#[test]
fn kept_lines_below_a_tall_first_page_header_move_whole_to_page_two() {
    use wordcraft_doc::props::LineSpacing;
    use wordcraft_doc::section::SectionStart;
    let exact = |p: &mut wordcraft_doc::Paragraph, h: f32| {
        p.props.line_spacing = Some(LineSpacing::Exactly(h));
        p.props.space_before = Some(0.0);
        p.props.space_after = Some(0.0);
        p.props.widow_control = Some(false);
    };
    let kept_text = "word ".repeat(300);
    // How tall the kept paragraph is (its line count depends on the fonts; 12 pt lines).
    let mut probe = Document::from_text(&kept_text);
    exact(probe.para_mut(StoryRef::Body, &Path::top(0)).unwrap(), 12.0);
    let pl = lay(&probe);
    let end = pl.caret(&Pos::body(0, kept_text.len())).unwrap();
    assert_eq!(end.page, 0, "{end:?}");
    let h = end.top + 12.0 - pl.caret(&Pos::body(0, 0)).unwrap().top;
    let mut d = Document::from_text(&format!("x\n{kept_text}"));
    let s = d.last_section.clone();
    let (bottom, full) = (s.page_h - s.margin_bottom, s.page_h - s.margin_bottom - s.margin_top);
    assert!(h > 120.0 && h + 12.0 < full, "kept paragraph {h} fits a plain page of {full}");
    // The first section's first-page header leaves page 1, below the 12 pt filler, room for all
    // but 36 pt of the kept paragraph.
    let header_h = bottom - 12.0 - (h - 36.0) - s.header;
    let mut head = wordcraft_doc::Paragraph::with_text("Title", Default::default());
    exact(&mut head, header_h);
    let id = d.add_part(wordcraft_doc::PartKind::Header, vec![wordcraft_doc::para_block(head)]);
    let mut first_sect = s.clone();
    first_sect.title_page = true;
    first_sect.headers.first = Some(id);
    let filler = d.para_mut(StoryRef::Body, &Path::top(0)).unwrap();
    exact(filler, 12.0);
    filler.section = Some(Box::new(first_sect));
    d.last_section.start = SectionStart::Continuous;
    let kept = d.para_mut(StoryRef::Body, &Path::top(1)).unwrap();
    exact(kept, 12.0);
    kept.props.keep_lines = Some(true);
    let l = lay(&d);
    let filler = l.caret(&Pos::body(0, 0)).unwrap();
    assert_eq!(filler.page, 0, "{filler:?}");
    assert!(filler.top > s.margin_top + 100.0, "the tall header pushes page 1's body down: {filler:?}");
    let (first, last) = (l.caret(&Pos::body(1, 0)).unwrap(), l.caret(&Pos::body(1, kept_text.len())).unwrap());
    assert_eq!((first.page, last.page), (1, 1), "kept paragraph whole on page 2: {first:?} .. {last:?}");
}

/// Sections starting continuously one after another still go onto new pages: a paragraph that
/// doesn't fit below the text above it moves on rather than running off the page (#3907). A
/// hundred one-line sections lay out like a hundred plain paragraphs.
#[test]
fn continuous_sections_running_off_a_page_go_onto_the_next() {
    use wordcraft_doc::section::{SectionProps, SectionStart};
    let marks = 100;
    let text = format!("Start\n{}End", "x\n".repeat(marks));
    let mut d = Document::from_text(&text);
    for i in 1..=marks {
        d.para_mut(StoryRef::Body, &Path::top(i)).unwrap().section =
            Some(Box::new(SectionProps { start: SectionStart::Continuous, ..Default::default() }));
    }
    d.last_section.start = SectionStart::Continuous;
    let end = lay(&d).caret(&Pos::body(marks + 1, 0)).unwrap();
    let plain = lay(&Document::from_text(&text)).caret(&Pos::body(marks + 1, 0)).unwrap();
    assert!(end.page > 0, "End past the first page: {end:?}");
    assert_eq!((end.page, (end.top * 10.0).round()), (plain.page, (plain.top * 10.0).round()), "sections {end:?}, plain {plain:?}");
}

/// Bulleted empty marks past a full page's bottom draw nothing there, not even their bullets;
/// those that fit keep theirs (#3907 review: a stray bullet below the column).
#[test]
fn stacked_marks_past_the_bottom_draw_no_label() {
    use wordcraft_doc::ListKind;
    use wordcraft_doc::section::{SectionProps, SectionStart};
    let marks = 100;
    let mut d = Document::from_text(&format!("Start\n{}End", "\n".repeat(marks)));
    let id = Arc::make_mut(&mut d.numbering).add_list(ListKind::Bullet);
    for i in 1..=marks {
        let p = d.para_mut(StoryRef::Body, &Path::top(i)).unwrap();
        p.section = Some(Box::new(SectionProps { start: SectionStart::Continuous, ..Default::default() }));
        p.props.numbering = Some(wordcraft_doc::props::NumRef { num: id, level: 0 });
    }
    d.last_section.start = SectionStart::Continuous;
    let l = lay(&d);
    let page = &l.pages[0];
    let bottom = page.body.y + page.body.h;
    let (mut inside, mut below) = (0, 0);
    for it in &page.items {
        let Placed::Lines { para, y, .. } = it else { continue };
        if para.label.is_none() {
            continue;
        }
        if *y < bottom - 0.01 { inside += 1 } else { below += 1 }
    }
    assert!(inside > 10, "marks that fit keep their bullets: {inside}");
    // Nor their line numbers (CodeRabbit).
    for i in 1..=marks {
        if let Some(s) = d.para_mut(StoryRef::Body, &Path::top(i)).unwrap().section.as_mut() {
            s.line_numbers = Some(Default::default());
        }
    }
    d.last_section.line_numbers = Some(Default::default());
    let numbered = lay(&d);
    let page = &numbered.pages[0];
    let low = page.items.iter().filter(|it| matches!(it, Placed::Lines { story, y, .. } if *story == DECOR_STORY && *y >= bottom - 0.01)).count();
    assert_eq!(low, 0, "no line number at or below the body's bottom ({bottom})");
    assert!(page.items.iter().any(|it| matches!(it, Placed::Lines { story, .. } if *story == DECOR_STORY)), "the marks that fit are numbered");
    assert_eq!(below, 0, "no bullet at or below the body's bottom ({bottom})");
    assert_eq!(l.caret(&Pos::body(marks + 1, 0)).unwrap().page, 1, "End still starts page 2");
}

/// An empty mark after a text paragraph that itself closes a continuous section takes no room,
/// as after any text: only marks after an empty mark stack (#3907 review).
#[test]
fn an_empty_mark_after_a_text_section_mark_stays_quiet() {
    use wordcraft_doc::section::{SectionProps, SectionStart};
    let gap = |text_closes: bool| {
        let mut d = Document::from_text("Start\n\nEnd");
        let cont = || Some(Box::new(SectionProps { start: SectionStart::Continuous, ..Default::default() }));
        if text_closes {
            d.para_mut(StoryRef::Body, &Path::top(0)).unwrap().section = cont();
        }
        d.para_mut(StoryRef::Body, &Path::top(1)).unwrap().section = cont();
        d.last_section.start = SectionStart::Continuous;
        let l = lay(&d);
        l.caret(&Pos::body(2, 0)).unwrap().top - l.caret(&Pos::body(0, 0)).unwrap().top
    };
    assert!((gap(true) - gap(false)).abs() < 0.01, "text closing a section: {}, plain text: {}", gap(true), gap(false));
}

/// dpkruse's Genuine Steps Certificate (#3907 review): a bulleted text paragraph closes a
/// two-column continuous section, then an empty bulleted paragraph closes another. Word draws no
/// bullet for the empty one wherever the text above ends, down to the foot of the page.
#[test]
fn an_empty_bulleted_mark_after_a_bulleted_section_closer_draws_no_bullet() {
    use wordcraft_doc::ListKind;
    use wordcraft_doc::section::{SectionProps, SectionStart};
    for filler in (40..140).step_by(3) {
        let mut d = Document::from_text(&format!("{}file this form\n\nFailure to comply", "Instructions\n".repeat(filler)));
        let id = Arc::make_mut(&mut d.numbering).add_list(ListKind::Bullet);
        let mut two = SectionProps { start: SectionStart::Continuous, ..Default::default() };
        two.columns.count = 2;
        for i in [filler, filler + 1] {
            let p = d.para_mut(StoryRef::Body, &Path::top(i)).unwrap();
            p.section = Some(Box::new(two.clone()));
            p.props.numbering = Some(wordcraft_doc::props::NumRef { num: id, level: 0 });
        }
        d.last_section.start = SectionStart::Continuous;
        let l = lay(&d);
        let bullets = l
            .pages
            .iter()
            .flat_map(|page| &page.items)
            .filter(|it| matches!(it, Placed::Lines { path, para, .. } if *path == Path::top(filler + 1) && para.label.is_some()))
            .count();
        assert_eq!(bullets, 0, "{filler} lines above: the empty mark draws no bullet");
        let text = l
            .pages
            .iter()
            .flat_map(|page| &page.items)
            .any(|it| matches!(it, Placed::Lines { path, para, .. } if *path == Path::top(filler) && para.label.is_some()));
        assert!(text, "{filler} lines above: the text closer keeps its bullet");
    }
}

/// A page break before the first row's paragraph moves the whole table to a new page (Word's
/// behaviour; an ODT table starting a page opens so), unless the table is at a page's top.
#[test]
fn a_page_break_in_a_tables_first_row_moves_the_table() {
    let mut d = Document::from_text("Before the table.");
    let mut table = Table::new(1, 1, 468.0);
    let mut first = wordcraft_doc::Paragraph::with_text("First cell", Default::default());
    first.props.page_break_before = Some(true);
    table.rows[0].cells[0].blocks = vec![wordcraft_doc::para_block(first)];
    d.body.push(std::sync::Arc::new(wordcraft_doc::Block::Table(table.clone())));
    let l = lay(&d);
    assert_eq!(l.pages.len(), 2, "the table starts page two");
    // At the top of the document it starts no extra page.
    d.body = vec![std::sync::Arc::new(wordcraft_doc::Block::Table(table))];
    assert_eq!(lay(&d).pages.len(), 1);
}

#[test]
fn a_cells_background_picture_is_drawn_clipped_to_the_cell() {
    // An ODF cell's background picture: once at its own size at the cell's top left, under the
    // text and cut off at the cell's edges; tiled, as many copies as cover the cell.
    use wordcraft_doc::props::{CellPicture, PictureRepeat};
    let mut d = Document::new();
    let mut t = wordcraft_doc::Table::new(1, 1, 200.0);
    let media = d.add_media(vec![0u8; 8], "png");
    let pic = |repeat| Some(Box::new(CellPicture { media: media.clone(), size: [30.0, 6.0], repeat, align: [0.0, 0.0] }));
    t.rows[0].cells[0].props.picture = pic(PictureRepeat::None);
    d.body = vec![std::sync::Arc::new(wordcraft_doc::Block::Table(t.clone())), wordcraft_doc::para_block(wordcraft_doc::Paragraph::new())];
    let clips = |l: &DocLayout| -> Vec<(Rect, Vec<Rect>)> {
        l.pages[0]
            .items
            .iter()
            .filter_map(|it| match it {
                Placed::Clip { rect, items, .. } => {
                    Some((*rect, items.iter().filter_map(|i| if let Placed::Image { rect, .. } = i { Some(*rect) } else { None }).collect()))
                }
                _ => None,
            })
            .collect()
    };
    let c = clips(&lay(&d));
    assert_eq!(c.len(), 1);
    let (cell, images) = &c[0];
    assert_eq!(images.len(), 1);
    assert!(
        (images[0].x - cell.x).abs() < 0.01 && (images[0].y - cell.y).abs() < 0.01 && (images[0].w - 30.0).abs() < 0.01,
        "{images:?} in {cell:?}"
    );
    t.rows[0].cells[0].props.picture = pic(PictureRepeat::Tile);
    d.body = vec![std::sync::Arc::new(wordcraft_doc::Block::Table(t)), wordcraft_doc::para_block(wordcraft_doc::Paragraph::new())];
    let c = clips(&lay(&d));
    let (cell, images) = &c[0];
    let want = (cell.w / 30.0).ceil() as usize * (cell.h / 6.0).ceil() as usize;
    assert_eq!(images.len(), want, "{cell:?}");
    // More tiles than are drawn (a small pattern in a big cell): stretched over the whole cell,
    // not tiled over its top rows only.
    let mut t = wordcraft_doc::Table::new(1, 1, 400.0);
    t.rows[0].cells[0].props.picture =
        Some(Box::new(CellPicture { media: media.clone(), size: [2.0, 1.0], repeat: PictureRepeat::Tile, align: [0.0, 0.0] }));
    d.body = vec![std::sync::Arc::new(wordcraft_doc::Block::Table(t)), wordcraft_doc::para_block(wordcraft_doc::Paragraph::new())];
    let c = clips(&lay(&d));
    let (cell, images) = &c[0];
    assert!((cell.w / 2.0).ceil() * (cell.h / 1.0).ceil() > 400.0, "{cell:?}");
    assert_eq!(images.len(), 1, "{cell:?}");
    let i = images[0];
    assert!(
        (i.x - cell.x).abs() < 0.01 && (i.y - cell.y).abs() < 0.01 && (i.w - cell.w).abs() < 0.01 && (i.h - cell.h).abs() < 0.01,
        "{i:?} in {cell:?}"
    );
}

#[test]
fn default_tab_stops_count_from_the_indent_in_odt_documents_that_say_so() {
    // LibreOffice with `TabsRelativeToIndent` (its default), measured: default stops every
    // interval from the paragraph's left indent; the hanging indent still a stop. Word, and ODT
    // documents measuring from the margin: from the margin.
    let x_of_b = |from_indent: bool, left: f32, first: f32| {
        let mut d = Document::from_text("a\tb");
        d.settings.default_tab = 36.0;
        d.settings.odf = Some(wordcraft_doc::OdfLayout { tabs_from_indent: from_indent });
        d.format_paragraphs(&Pos::body(0, 0), &Pos::body(0, 0), &|p| {
            p.indent_left = Some(left);
            p.indent_first = Some(first);
        })
        .unwrap();
        let l = lay(&d);
        l.caret(&Pos::body(0, 2)).unwrap().x - 72.0
    };
    assert!((x_of_b(true, 17.0, 0.0) - 53.0).abs() < 0.5, "{}", x_of_b(true, 17.0, 0.0));
    assert!((x_of_b(false, 17.0, 0.0) - 36.0).abs() < 0.5, "{}", x_of_b(false, 17.0, 0.0));
    // Hanging: the indent is the first stop either way.
    assert!((x_of_b(true, 85.0, -85.0) - 85.0).abs() < 0.5, "{}", x_of_b(true, 85.0, -85.0));
}

#[test]
fn a_list_item_that_shows_nothing_reports_no_missing_font() {
    // Its number is shaped in its mark's font before the paragraph turns out to show nothing
    // (#3982): that font isn't shown in another, so it isn't missing from what's drawn.
    let mut d = Document::from_text("Alpha\n\nBeta");
    let id = std::sync::Arc::make_mut(&mut d.numbering).add_list(wordcraft_doc::ListKind::Numbered);
    d.format_paragraphs(&Pos::body(0, 0), &Pos::body(2, 4), &|p| {
        p.numbering = Some(wordcraft_doc::props::NumRef { num: id, level: 0 });
    })
    .unwrap();
    let mark = &mut d.para_mut(StoryRef::Body, &Path::top(1)).unwrap().mark;
    mark.font = Some("Wordcraft Gone Number Face".into());
    let missing = |d: &Document| layout(d, &mut LayoutCache::new(), &LayoutOptions::default()).substituted_fonts();
    let shown = missing(&d);
    assert!(shown.iter().any(|(f, _)| f == "Wordcraft Gone Number Face"), "a shown number counts: {shown:?}");
    d.para_mut(StoryRef::Body, &Path::top(1)).unwrap().mark.hidden = Some(true);
    let gone = missing(&d);
    assert!(!gone.iter().any(|(f, _)| f == "Wordcraft Gone Number Face"), "{gone:?}");
}

fn boundary_header_para() -> Paragraph {
    let mut p = Paragraph::with_text("One\nTwo\nThree\nFour\nFive", Default::default());
    p.props.line_spacing = Some(wordcraft_doc::props::LineSpacing::Exactly(10.0));
    p.props.space_before = Some(0.0);
    p.props.space_after = Some(0.0);
    p
}

#[test]
fn header_boundary_pinned() {
    use wordcraft_doc::para::{Anchor, Float, FloatAlign, Wrap};
    for anchor in [Anchor::Margin, Anchor::Page, Anchor::TopMargin] {
        for align in [None, Some(FloatAlign::Center)] {
            let mut doc = Document::from_text("Body");
            doc.last_section.margin_top = 60.0;
            doc.last_section.header = 36.0;
            let mut cell = Paragraph::with_text("Cell", Default::default());
            let float = Float {
                wrap: Wrap::BehindText,
                h_rel: Anchor::Page,
                v_rel: anchor,
                v_align: align,
                outside_cell: true,
                x: 300.0,
                y: -20.0,
                ..Default::default()
            };
            cell.insert_object(0, rect_shape(40.0, 10.0, float), &Default::default()).unwrap();
            let mut table = Table::new(1, 1, 40.0);
            table.rows[0].cells[0].blocks = vec![wordcraft_doc::para_block(cell)];
            let header = doc.add_part(
                wordcraft_doc::PartKind::Header,
                vec![wordcraft_doc::para_block(boundary_header_para()), Arc::new(Block::Table(table.clone()))],
            );
            let footer = doc.add_part(wordcraft_doc::PartKind::Footer, vec![Arc::new(Block::Table(table))]);
            doc.last_section.headers.default = Some(header);
            doc.last_section.footers.default = Some(footer);
            let laid = lay(&doc);
            let page = &laid.pages[0];
            assert!(page.body.y > doc.last_section.margin_top);
            for (items, margin) in [(&page.header, page.body.y), (&page.footer, doc.last_section.margin_top)] {
                let (start, extent) = match anchor {
                    Anchor::Margin => (margin, doc.last_section.page_h - margin - doc.last_section.margin_bottom),
                    Anchor::TopMargin => (0.0, doc.last_section.margin_top),
                    _ => (0.0, doc.last_section.page_h),
                };
                let expected = match align {
                    Some(FloatAlign::Center) => start + (extent - 10.0) / 2.0,
                    _ => start - 20.0,
                };
                let rects = shapes(items);
                assert_eq!(rects.len(), 1);
                assert!((rects[0].y - expected).abs() < 0.01, "{anchor:?} {align:?}: {:?}, expected {expected}, body {:?}", rects[0], page.body);
                let hit = items
                    .iter()
                    .find_map(|item| match item {
                        Placed::Object { rect, .. } => Some(rect),
                        _ => None,
                    })
                    .unwrap();
                assert!((hit.y - expected).abs() < 0.01);
            }
        }
    }
}

#[test]
fn header_boundary_obstacle() {
    use wordcraft_doc::para::Anchor;
    use wordcraft_doc::props::{HeightRule, TableFloat};
    const FLOAT_HEIGHT: f32 = 10.0;
    for overlap in [false, true] {
        let mut doc = Document::from_text("Body");
        doc.last_section.margin_top = 60.0;
        doc.last_section.header = 36.0;
        let mut header_table = Table::new(1, 1, 40.0);
        header_table.rows[0].props.height = Some(FLOAT_HEIGHT);
        header_table.rows[0].props.height_rule = HeightRule::Exact;
        header_table.props.float = Some(TableFloat { h_rel: Anchor::Page, v_rel: Anchor::Margin, x: 300.0, y: -20.0, ..Default::default() });
        let id = doc
            .add_part(wordcraft_doc::PartKind::Header, vec![wordcraft_doc::para_block(boundary_header_para()), Arc::new(Block::Table(header_table))]);
        doc.last_section.headers.default = Some(id);
        let target = lay(&doc).pages[0].body.y - 20.0;
        let mut body_table = Table::new(1, 1, 40.0);
        body_table.rows[0].props.height = Some(FLOAT_HEIGHT);
        body_table.rows[0].props.height_rule = HeightRule::Exact;
        body_table.props.float = Some(TableFloat { h_rel: Anchor::Page, v_rel: Anchor::Page, x: 300.0, y: target, overlap, ..Default::default() });
        doc.body.insert(0, Arc::new(Block::Table(body_table)));
        let laid = lay(&doc);
        let page = &laid.pages[0];
        let cell = |items: &[Placed]| {
            items
                .iter()
                .find_map(|item| match item {
                    Placed::Cell { rect, .. } => Some(*rect),
                    _ => None,
                })
                .unwrap()
        };
        let (header, body) = (cell(&page.header), cell(&page.items));
        assert!(page.body.y > doc.last_section.margin_top);
        assert!((header.y - target).abs() < 0.01);
        if overlap {
            assert!((body.y - target).abs() < 0.01, "overlap retains requested position: {body:?}");
        } else {
            assert!(body.y >= header.bottom() - 0.01, "body {body:?} overlaps rendered header {header:?}");
        }
    }
}

#[test]
fn header_margin_tracks_body_top() {
    use wordcraft_doc::para::{Anchor, Float, Wrap};
    use wordcraft_doc::props::LineSpacing;
    for margin in [60.0, 120.0] {
        for anchor in [Anchor::Margin, Anchor::Page, Anchor::Paragraph] {
            let mut d = Document::from_text("Body");
            d.last_section.margin_top = margin;
            d.last_section.header = 36.0;
            let mut hp = Paragraph::with_text("One\nTwo\nThree\nFour\nFive", Default::default());
            hp.props.line_spacing = Some(LineSpacing::Exactly(10.0));
            hp.props.space_before = Some(0.0);
            hp.props.space_after = Some(0.0);
            let float = Float { wrap: Wrap::BehindText, h_rel: Anchor::Page, v_rel: anchor, x: 300.0, y: -20.0, ..Default::default() };
            hp.insert_object(0, rect_shape(40.0, 10.0, float), &Default::default()).unwrap();
            let id = d.add_part(wordcraft_doc::PartKind::Header, vec![wordcraft_doc::para_block(hp)]);
            d.last_section.headers.default = Some(id);
            let l = lay(&d);
            let page = &l.pages[0];
            let expected = match anchor {
                Anchor::Margin => page.body.y - 20.0,
                Anchor::Page => -20.0,
                _ => 16.0,
            };
            assert!(
                (shapes(&page.header)[0].y - expected).abs() < 0.01,
                "margin {margin}, anchor {anchor:?}: {:?}, body {:?}",
                shapes(&page.header),
                page.body
            );
        }
    }
}

#[test]
fn header_anchor_extents_stay() {
    use wordcraft_doc::para::{Anchor, Float, FloatAlign, Wrap};
    use wordcraft_doc::props::LineSpacing;
    for margin in [60.0, 120.0] {
        for anchor in
            [Anchor::Margin, Anchor::TopMargin, Anchor::InsideMargin, Anchor::Page, Anchor::Paragraph, Anchor::BottomMargin, Anchor::OutsideMargin]
        {
            for align in [None, Some(FloatAlign::Start), Some(FloatAlign::Center), Some(FloatAlign::End)] {
                let mut d = Document::from_text("Body");
                d.last_section.margin_top = margin;
                d.last_section.header = 36.0;
                let mut hp = Paragraph::with_text("One\nTwo\nThree\nFour\nFive", Default::default());
                hp.props.line_spacing = Some(LineSpacing::Exactly(10.0));
                hp.props.space_before = Some(0.0);
                hp.props.space_after = Some(0.0);
                let float =
                    Float { wrap: Wrap::BehindText, h_rel: Anchor::Page, v_rel: anchor, v_align: align, x: 300.0, y: -20.0, ..Default::default() };
                hp.insert_object(0, rect_shape(40.0, 10.0, float), &Default::default()).unwrap();
                let id = d.add_part(wordcraft_doc::PartKind::Header, vec![wordcraft_doc::para_block(hp)]);
                d.last_section.headers.default = Some(id);
                let l = lay(&d);
                let page = &l.pages[0];
                if margin == 60.0 {
                    assert!(page.body.y > margin, "the header must displace the body");
                } else {
                    assert!((page.body.y - margin).abs() < 0.01, "the header must fit within the declared margin");
                }
                let s = &d.last_section;
                let (start, extent) = match anchor {
                    Anchor::Margin => (page.body.y, s.page_h - page.body.y - s.margin_bottom),
                    Anchor::TopMargin | Anchor::InsideMargin => (0.0, margin),
                    Anchor::Page => (0.0, s.page_h),
                    Anchor::BottomMargin | Anchor::OutsideMargin => (s.page_h - s.margin_bottom, s.margin_bottom),
                    _ => (s.header, 0.0),
                };
                let expected = match align {
                    None => start - 20.0,
                    Some(FloatAlign::Center) => start + (extent - 10.0) / 2.0,
                    Some(FloatAlign::End) => start + extent - 10.0,
                    _ => start,
                };
                let drawn = shapes(&page.header);
                assert_eq!(drawn.len(), 1, "the countercontrol must place its drawing");
                assert!(
                    (drawn[0].y - expected).abs() < 0.01,
                    "margin {margin}, anchor {anchor:?}, align {align:?}: {:?}, expected {expected}, body {:?}",
                    drawn,
                    page.body
                );
            }
        }
    }
}

#[test]
fn header_margin_table_area() {
    use wordcraft_doc::para::{Anchor, FloatAlign};
    use wordcraft_doc::props::{HeightRule, LineSpacing, TableFloat};
    for margin in [60.0, 120.0] {
        for align in [None, Some(FloatAlign::Center), Some(FloatAlign::End)] {
            let mut d = Document::from_text("Body");
            d.last_section.margin_top = margin;
            d.last_section.header = 36.0;
            let mut hp = Paragraph::with_text("One\nTwo\nThree\nFour\nFive", Default::default());
            hp.props.line_spacing = Some(LineSpacing::Exactly(10.0));
            hp.props.space_before = Some(0.0);
            hp.props.space_after = Some(0.0);
            let mut t = Table::new(1, 1, 40.0);
            t.rows[0].props.height = Some(10.0);
            t.rows[0].props.height_rule = HeightRule::Exact;
            t.props.float = Some(TableFloat { h_rel: Anchor::Page, v_rel: Anchor::Margin, v_align: align, x: 300.0, y: -20.0, ..Default::default() });
            let id = d.add_part(wordcraft_doc::PartKind::Header, vec![wordcraft_doc::para_block(hp), Arc::new(Block::Table(t))]);
            d.last_section.headers.default = Some(id);
            let l = lay(&d);
            let page = &l.pages[0];
            if margin == 60.0 {
                assert!(page.body.y > margin, "the header must displace the body");
            }
            let cells: Vec<_> = page
                .header
                .iter()
                .filter_map(|it| match it {
                    Placed::Cell { rect, .. } => Some(rect),
                    _ => None,
                })
                .collect();
            assert_eq!(cells.len(), 1);
            let extent = (d.last_section.page_h - page.body.y - d.last_section.margin_bottom).max(18.0);
            let expected = match align {
                Some(FloatAlign::Center) => page.body.y + (extent - 10.0) / 2.0,
                Some(FloatAlign::End) => page.body.y + extent - 10.0,
                _ => page.body.y - 20.0,
            };
            assert!((cells[0].y - expected).abs() < 0.01, "margin {margin}, align {align:?}: {:?}, expected {expected}", cells[0]);
        }
    }
}

mod shape_alpha;

/// A row with a background picture split over two pages: each part draws its own piece of the
/// picture inside its own part of the cell, and the text moving on starts at the continuation's
/// top (the picture is the cell's background, not content moving on with the text).
#[test]
fn a_split_cells_background_picture_is_drawn_on_both_parts() {
    use wordcraft_doc::props::{CellPicture, PictureRepeat};
    let mut d = Document::new();
    let mut t = wordcraft_doc::Table::new(1, 1, 200.0);
    let media = d.add_media(vec![0u8; 8], "png");
    t.rows[0].cells[0].props.picture = Some(Box::new(CellPicture { media, size: [30.0, 6.0], repeat: PictureRepeat::Tile, align: [0.0, 0.0] }));
    t.rows[0].cells[0].blocks =
        (0..90).map(|i| wordcraft_doc::para_block(wordcraft_doc::Paragraph::with_text(&format!("Line {i}"), Default::default()))).collect();
    d.body = vec![std::sync::Arc::new(wordcraft_doc::Block::Table(t)), wordcraft_doc::para_block(wordcraft_doc::Paragraph::new())];
    let l = lay(&d);
    assert!(l.pages.len() >= 2, "the row splits");
    for (n, page) in l.pages.iter().take(2).enumerate() {
        let clips: Vec<Rect> = page.items.iter().filter_map(|it| if let Placed::Clip { rect, .. } = it { Some(*rect) } else { None }).collect();
        assert_eq!(clips.len(), 1, "page {n}: {clips:?}");
        let clip = clips[0];
        let cells: Vec<Rect> = page.items.iter().filter_map(|it| if let Placed::Cell { rect, .. } = it { Some(*rect) } else { None }).collect();
        assert!(cells.iter().any(|c| (c.y - clip.y).abs() < 0.01 && (c.h - clip.h).abs() < 0.01), "page {n}: {clip:?} is its cell part {cells:?}");
        assert!(clip.y >= 0.0 && clip.bottom() <= page.h + 0.01, "page {n}: {clip:?}");
        // The text starts near the top of its part of the cell.
        let first = page.items.iter().find_map(|it| if let Placed::Lines { y, .. } = it { Some(*y) } else { None }).unwrap_or(f32::MAX);
        assert!(first - clip.y < 20.0, "page {n}: text at {first}, cell at {clip:?}");
    }
}

/// A table moved to a new page by a page break before its first paragraph shows the number of
/// the page it lands on in that cell's PAGE field, not the page it was laid out for.
#[test]
fn a_table_moved_by_a_page_break_shows_its_new_page_number() {
    let mut d = Document::from_text("Before the table.");
    let mut table = Table::new(1, 1, 468.0);
    let mut first = wordcraft_doc::Paragraph::with_text("Page ", Default::default());
    first.props.page_break_before = Some(true);
    let page = InlineObject::Field { instr: "PAGE".into(), result: "9".into(), locked: false, dirty: true, code: false, form: String::new() };
    first.insert_object(first.len(), page, &Default::default()).unwrap();
    table.rows[0].cells[0].blocks = vec![wordcraft_doc::para_block(first)];
    d.body.push(std::sync::Arc::new(wordcraft_doc::Block::Table(table)));
    let l = lay(&d);
    assert_eq!(l.pages.len(), 2);
    let shown: Vec<String> = l.pages[1]
        .items
        .iter()
        .flat_map(|it| match it {
            Placed::Lines { para, .. } => para.shown.iter().map(|s| s.1.clone()).collect::<Vec<_>>(),
            _ => Vec::new(),
        })
        .collect();
    assert_eq!(shown, ["2"]);
}

/// Cell confinement must not change page-vertical placement through the rotated text axis.
#[test]
fn turned_cell_float_keeps_its_along_text_offset() {
    use wordcraft_doc::para::{Anchor, Float, Wrap};
    let place = |direction, outside_cell| {
        let mut doc = Document::from_text("Synthetic body");
        let mut table = Table::new(1, 2, 468.0);
        table.rows[0].props.height = Some(100.0);
        table.rows[0].props.height_rule = wordcraft_doc::props::HeightRule::Exact;
        table.rows[0].cells[0].props.text_direction = direction;
        let mut para = Paragraph::with_text("Synthetic cell", Default::default());
        let float = Float { wrap: Wrap::Tight, h_rel: Anchor::Column, v_rel: Anchor::Paragraph, x: 150.0, outside_cell, ..Default::default() };
        para.insert_object(0, rect_shape(20.0, 10.0, float), &Default::default()).unwrap();
        table.rows[0].cells[0].blocks = vec![wordcraft_doc::para_block(para)];
        doc.insert_block(StoryRef::Body, &Path::top(1), Block::Table(table)).unwrap();
        let objects = shapes(&lay(&doc).pages[0].items);
        assert_eq!(objects.len(), 1);
        objects[0]
    };
    for direction in [TextDirection::Down, TextDirection::Up] {
        let confined = place(direction, false);
        let reference = place(direction, true);
        assert!((confined.y - reference.y).abs() < 0.01, "{direction:?}: {confined:?} vs {reference:?}");
    }
}
