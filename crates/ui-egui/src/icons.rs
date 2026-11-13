//! Letra's icons. Every button icon is NDL's (nodalia-design's shared set, drawn in its bold
//! style); [`ndl_name`] maps Letra's names onto it. Only the previews of insertable shapes
//! (`shape.*`) are drawn here, on a 20×20 grid in line work (`c`) and an accent (`a`).

use egui::{Color32, Painter, Pos2, Rect, Shape, Stroke, pos2, vec2};

struct Pen<'a> {
    p: &'a Painter,
    r: Rect,
    c: Color32,
    a: Color32,
    w: f32,
}

impl Pen<'_> {
    fn s(&self) -> f32 {
        self.r.width().min(self.r.height()) / 20.0
    }
    fn pt(&self, x: f32, y: f32) -> Pos2 {
        let s = self.s();
        let o = self.r.center() - vec2(10.0 * s, 10.0 * s);
        pos2(o.x + x * s, o.y + y * s)
    }
    fn stroke(&self, c: Color32) -> Stroke {
        Stroke::new(self.w * self.s(), c)
    }
    fn line(&self, pts: &[(f32, f32)]) {
        self.line_c(pts, self.c);
    }
    fn line_c(&self, pts: &[(f32, f32)], c: Color32) {
        let v: Vec<Pos2> = pts.iter().map(|&(x, y)| self.pt(x, y)).collect();
        self.p.add(Shape::line(v, self.stroke(c)));
    }
    fn closed(&self, pts: &[(f32, f32)], c: Color32) {
        let v: Vec<Pos2> = pts.iter().map(|&(x, y)| self.pt(x, y)).collect();
        self.p.add(Shape::closed_line(v, self.stroke(c)));
    }
    fn fill(&self, pts: &[(f32, f32)], c: Color32) {
        let v: Vec<Pos2> = pts.iter().map(|&(x, y)| self.pt(x, y)).collect();
        self.p.add(Shape::convex_polygon(v, c, Stroke::NONE));
    }
    fn fcircle(&self, x: f32, y: f32, r: f32, c: Color32) {
        self.p.circle_filled(self.pt(x, y), r * self.s(), c);
    }
}

/// The NDL icon drawn for Letra's icon `name`. Every icon Letra draws comes from NDL's shared set.
pub fn ndl_name(name: &str) -> Option<&'static str> {
    Some(match name {
        "accept" => "change-accept",
        "accessibility" => "accessibility",
        "addins" => "package",
        "addNote" => "comment-add",
        "addPen" => "pen",
        "addressBlock" => "address-block",
        "addShape" => "shapes",
        "addText" => "add-text",
        "align" => "align-objects-left",
        "alignCenter" => "align-center",
        "alignLeft" => "align-left",
        "alignRight" => "align-right",
        "alignText" => "align-text",
        "altText" => "alt-text",
        "arrangeAll" => "grid-view",
        "arrow" => "arrow-right",
        "autofit" => "fit",
        "bibliography" => "book",
        "blankPage" => "new",
        "blockAuthors" => "block-authors",
        "borderPainter" => "format-painter",
        "bold" => "bold",
        "bookmark" => "bookmark",
        "borders" => "borders",
        "breaks" => "page-break",
        "bringForward" => "bring-forward",
        "bullets" => "list",
        "canvas" => "rectangle",
        "caption" => "caption",
        "case" => "change-case",
        "ccCheckBox" => "check-square",
        "ccComboBox" => "content-combo-box",
        "ccDate" => "calendar",
        "ccDropDown" => "content-drop-down",
        "ccGallery" => "grid-view",
        "ccPicture" => "image",
        "ccPlainText" => "text",
        "ccRepeat" => "content-repeat",
        "ccRichText" => "text-box",
        "cellAlign" => "align-text",
        "cellMargins" => "margins",
        "changeChartType" => "chart",
        "charborder" => "borders",
        "chart" => "chart",
        "chartData" => "database",
        "chartTitle" => "heading",
        "check" => "check",
        "checkErrors" => "spellcheck",
        "chevronDoubleRight" => "chevron-double-right",
        "chevronDown" => "chevron-down",
        "chevronLeft" => "chevron-left",
        "chevronRight" => "chevron-right",
        "chevronUp" => "chevron-up",
        "citation" => "citation",
        "clear" => "clear-formatting",
        "close" => "close",
        "closeOutline" => "close",
        "colors" => "palette",
        "columns" => "columns",
        "colWidth" => "column-width",
        "combine" => "compare",
        "comment" => "comment",
        "compare" => "compare",
        "copy" => "copy",
        "coverPage" => "cover-page",
        "crop" => "crop",
        "crossRef" => "cross-reference",
        "curvedArrowConnector" => "connector-curved-arrow",
        "curvedConnector" => "connector-curved",
        "cut" => "cut",
        "darkMode" => "moon",
        "dataLabels" => "tag",
        "dateTime" => "calendar",
        "deleteComment" => "comment-delete",
        "deleteTable" => "delete",
        "demote" => "indent",
        "designMode" => "ruler",
        "diamond" => "diamond",
        "dictate" => "microphone",
        "distributeCols" => "distribute",
        "distributeRows" => "distribute",
        "docPrefs" => "settings",
        "documentTemplate" => "file",
        "draft" => "file-text",
        "draw" => "brush",
        "drawTable" => "pen",
        "dropCap" => "drop-cap",
        "dropdown" => "chevron-down",
        "editor" => "spellcheck",
        "editRecipients" => "users",
        "effects" => "text-effects",
        "effectsDesign" => "sparkle",
        "elbowArrowConnector" => "connector-elbow-arrow",
        "elbowConnector" => "connector-elbow",
        "ellipse" => "ellipse",
        "enclose" => "enclose-character",
        "endnote" => "endnote",
        "envelope" => "mail",
        "equation" => "equation",
        "eraser" => "eraser",
        "export" => "export",
        "file" => "file",
        "find" => "search",
        "finish" => "send",
        "first" => "skip-previous",
        "focus" => "focus-mode",
        "folder" => "folder",
        "fontcolor" => "text-color",
        "fontdialog" => "font-size",
        "fontMissing" => "warning",
        "fonts" => "text",
        "footer" => "footer",
        "footnote" => "footnote",
        "formula" => "sum",
        "fullScreen" => "fullscreen",
        "greetingLine" => "greeting-line",
        "gridlines" => "gridlines",
        "group" => "group",
        "grow" => "grow-font",
        "header" => "header",
        "heart" => "heart",
        "help" => "help",
        "hideInk" => "eye-off",
        "highlight" => "highlight",
        "highlightFields" => "highlight",
        "highlightNoColor" => "highlight",
        "homeFolder" => "home",
        "hyphenation" => "hyphenation",
        "icons" => "insert-icon",
        "image" => "image",
        "immersive" => "book",
        "indent" => "indent",
        "indentLeft" => "indent-left",
        "indentRight" => "indent-right",
        "index" => "index",
        "info" => "info",
        "inkToMath" => "equation",
        "inkToShape" => "shapes",
        "insertAbove" => "row-insert",
        "insertBelow" => "row-insert",
        "insertLeft" => "column-insert",
        "insertRight" => "column-insert",
        "interfaceTheme" => "contrast",
        "italic" => "italic",
        "justify" => "align-justify",
        "labels" => "tag",
        "language" => "globe",
        "lasso" => "lasso",
        "last" => "skip-next",
        "launcher" => "dialog-launcher",
        "legend" => "list",
        "line" => "line",
        "lineNumbers" => "line-numbers",
        "lineSpacing" => "line-spacing",
        "link" => "link",
        "macros" => "code",
        "mailMerge" => "mail",
        "margins" => "margins",
        "markCitation" => "quote",
        "markEntry" => "tag",
        "markup" => "markup",
        "matchFields" => "match-fields",
        "mediaNext" => "skip-next",
        "mediaPause" => "pause",
        "mediaPlay" => "play",
        "mediaPrev" => "skip-previous",
        "mediaStop" => "stop",
        "merge" => "merge-cells",
        "mergeField" => "merge-field",
        "minus" => "minus",
        "models3d" => "cube",
        "more" => "more-horizontal",
        "multilevel" => "list-multilevel",
        "multiplePages" => "pages",
        "navPane" => "sidebar",
        "newComment" => "comment-add",
        "newWindow" => "window",
        "next" => "chevron-right",
        "nextChange" => "change-next",
        "nextComment" => "comment-next",
        "nextFootnote" => "footnote",
        "numbering" => "list-numbers",
        "object" => "object",
        "onePage" => "file",
        "onlinePicture" => "online-picture",
        "open" => "open",
        "options" => "settings",
        "orientation" => "orientation",
        "outdent" => "outdent",
        "outline" => "list",
        "outlineBody" => "paragraph",
        "outlineCollapse" => "minus",
        "outlineDemote" => "indent",
        "outlineDown" => "arrow-down",
        "outlineExpand" => "plus",
        "outlinePromote" => "outdent",
        "outlineTop" => "move-top",
        "outlineUp" => "arrow-up",
        "pageBorders" => "borders",
        "pageBreak" => "page-break",
        "pageColor" => "page-color",
        "pageNumber" => "page-number",
        "pageWidth" => "fit-width",
        "painter" => "format-painter",
        "paraSpacing" => "line-spacing",
        "pastCitation" => "quote",
        "paste" => "paste",
        "pen" => "pen",
        "pencil" => "pen",
        "phonetic" => "text",
        "picture" => "image",
        "pilcrow" => "paragraph",
        "pin" => "pin",
        "plus" => "plus",
        "position" => "position",
        "prevChange" => "change-previous",
        "prevComment" => "comment-previous",
        "preview" => "preview",
        "previous" => "chevron-left",
        "print" => "print",
        "printLayout" => "file-text",
        "promote" => "outdent",
        "properties" => "properties",
        "protect" => "shield",
        "quickParts" => "quick-parts",
        "readAloud" => "volume",
        "readMode" => "book",
        "recipients" => "users",
        "recover" => "history",
        "rectangle" => "rectangle",
        "redo" => "redo",
        "reject" => "change-reject",
        "repeatHeader" => "repeat-header",
        "replace" => "replace",
        "replay" => "play",
        "replayForward" => "fast-forward",
        "replayPause" => "pause",
        "replayPlay" => "play",
        "replayRewind" => "rewind",
        "researcher" => "search",
        "resetGraphic" => "refresh",
        "resolve" => "check",
        "restrict" => "lock",
        "reviewingPane" => "panel-right",
        "rotate" => "rotate",
        "roundedRectangle" => "rounded-rectangle",
        "rowHeight" => "row-height",
        "ruler" => "ruler",
        "rules" => "list",
        "save" => "save",
        "saveAs" => "save-as",
        "screenshot" => "camera",
        "search" => "search",
        "select" => "select",
        "selectionPane" => "panel-right",
        "selectTable" => "select",
        "sendBackward" => "send-backward",
        "setDefault" => "check",
        "shading" => "fill",
        "shadingNoBar" => "fill",
        "shapeEffects" => "sparkle",
        "shapes" => "shapes",
        "share" => "share",
        "showComments" => "comments",
        "showNotes" => "footnote",
        "shrink" => "shrink-font",
        "sideBySide" => "columns",
        "sideToSide" => "columns",
        "signature" => "signature",
        "size" => "page-size",
        "smartArt" => "smart-art",
        "smartArtColors" => "palette",
        "sort" => "sort",
        "sources" => "sources",
        "spaceAfter" => "space-after",
        "spaceBefore" => "space-before",
        "spelling" => "spellcheck",
        "split" => "split-window",
        "splitCells" => "split-cells",
        "splitTable" => "split-table",
        "star" => "star",
        "straightArrowConnector" => "arrow-right",
        "straightConnector" => "line",
        "strike" => "strikethrough",
        "styles" => "styles",
        "stylesPane" => "panel-right",
        "subscript" => "subscript",
        "superscript" => "superscript",
        "switchWindows" => "switcher",
        "symbol" => "special-character",
        "syncScroll" => "sync-scroll",
        "table" => "table",
        "tableOfAuthorities" => "table-of-authorities",
        "tableOfFigures" => "table-of-figures",
        "textBox" => "text-box",
        "textBoxLink" => "link",
        "textBoxUnlink" => "unlink",
        "textDirection" => "text-direction",
        "textFill" => "text-color",
        "textLtr" => "text-left-to-right",
        "textOutline" => "text-outline",
        "textPane" => "sidebar",
        "textRtl" => "text-right-to-left",
        "themes" => "palette",
        "thesaurus" => "thesaurus",
        "thickness" => "lineweight",
        "toc" => "table-of-contents",
        "toText" => "text",
        "trackChanges" => "track-changes",
        "transform" => "transform",
        "translate" => "globe",
        "triangle" => "triangle",
        "underline" => "underline",
        "undo" => "undo",
        "unlinkCitations" => "unlink",
        "update" => "refresh",
        "updateTable" => "refresh",
        "user" => "user",
        "vertical" => "list-view",
        "video" => "video-camera",
        "watermark" => "stamp",
        "webLayout" => "globe",
        "wordArt" => "word-art",
        "wordCount" => "hash",
        "wrapText" => "wrap-text",
        "zoom" => "zoom",
        "zoom100" => "zoom",
        "zoomIn" => "zoom",
        "zoomOut" => "zoom-out",
        _ => return None,
    })
}

/// Paint NDL's icon `ndl` (its kebab-case name) into `r` in NDL's bold style. False when NDL has
/// no such icon.
pub fn paint_ndl(p: &Painter, r: Rect, ndl: &str, c: Color32) -> bool {
    nodalia_theme::icons::get(ndl).is_some_and(|icon| nodalia_theme::icons::paint(p, r, icon, nodalia_theme::icons::Style::bold(c)))
}

/// The colour bar under an icon whose button shows a colour (highlight, font colour, shading):
/// the default the button applies until one is picked.
fn bar(name: &str) -> Option<Color32> {
    match name {
        "highlight" | "shading" => Some(Color32::from_rgb(0xF2, 0xC8, 0x11)),
        "fontcolor" => Some(Color32::from_rgb(0xD1, 0x3B, 0x3B)),
        _ => None,
    }
}

/// Paint icon `name` into `r`. `c` is the line colour; the accent colours shape previews. Every
/// icon comes from NDL's shared set ([`ndl_name`]); `shape.*` are previews of insertable shapes.
pub fn paint(p: &Painter, r: Rect, name: &str, c: Color32, accent: Color32) {
    if let Some(ndl) = ndl_name(name) {
        match bar(name) {
            Some(colour) => {
                // The glyph sits above the bar, the way the button reads: tool, then colour.
                let glyph = Rect::from_min_max(r.min, pos2(r.max.x, r.max.y - r.height() * 0.2));
                let glyph = Rect::from_center_size(glyph.center(), vec2(glyph.height(), glyph.height()));
                paint_ndl(p, glyph, ndl, c);
                let y0 = r.min.y + r.height() * 0.825;
                let bar = Rect::from_min_max(pos2(r.min.x + r.width() * 0.1, y0), pos2(r.max.x - r.width() * 0.1, y0 + r.height() * 0.125));
                p.rect_filled(bar, 0.0, colour);
            }
            None => {
                paint_ndl(p, r, ndl, c);
            }
        }
        return;
    }
    let pen = Pen { p, r, c, a: accent, w: 1.25 };
    if let Some(kind) = name.strip_prefix("shape.") {
        shape(&pen, kind);
    }
}

/// The Insert › Shapes menu's icons: each shape kind (`insert.shape` ids) as a small outline.
fn shape(pen: &Pen, kind: &str) -> bool {
    let (c, a) = (pen.c, pen.a);
    let tint = a.linear_multiply(0.25);
    let solid = |pts: &[(f32, f32)]| {
        pen.fill(pts, tint);
        pen.closed(pts, c);
    };
    let head = |tip: (f32, f32), from: (f32, f32)| {
        let (dx, dy) = (tip.0 - from.0, tip.1 - from.1);
        let len = (dx * dx + dy * dy).sqrt().max(0.01);
        let (ux, uy) = (dx / len, dy / len);
        let back = (tip.0 - ux * 4.5, tip.1 - uy * 4.5);
        pen.fill(&[tip, (back.0 - uy * 2.5, back.1 + ux * 2.5), (back.0 + uy * 2.5, back.1 - ux * 2.5)], c);
    };
    let ends = |pts: &[(f32, f32)], arrow: bool| {
        pen.line(pts);
        if let Some(&first) = pts.first() {
            pen.fcircle(first.0, first.1, 1.6, a);
        }
        if let [.., from, tip] = pts {
            if arrow {
                head(*tip, *from);
            } else {
                pen.fcircle(tip.0, tip.1, 1.6, a);
            }
        }
    };
    let curve = || -> Vec<(f32, f32)> {
        let (p0, p1, p2, p3) = ((4.0, 15.0), (12.0, 15.0), (8.0, 5.0), (16.0, 5.0));
        (0..=16)
            .map(|i| {
                let s = i as f32 / 16.0;
                let m = 1.0 - s;
                let w = [m * m * m, 3.0 * m * m * s, 3.0 * m * s * s, s * s * s];
                (w[0] * p0.0 + w[1] * p1.0 + w[2] * p2.0 + w[3] * p3.0, w[0] * p0.1 + w[1] * p1.1 + w[2] * p2.1 + w[3] * p3.1)
            })
            .collect()
    };
    match kind {
        "rectangle" => solid(&[(3.0, 5.0), (17.0, 5.0), (17.0, 15.0), (3.0, 15.0)]),
        "roundedRectangle" => {
            let r = Rect::from_min_max(pen.pt(3.0, 5.0), pen.pt(17.0, 15.0));
            let radius = 3.0 * pen.s();
            pen.p.rect_filled(r, radius, tint);
            pen.p.rect_stroke(r, radius, pen.stroke(c), egui::StrokeKind::Middle);
        }
        "ellipse" => {
            let (center, radius) = (pen.pt(10.0, 10.0), vec2(7.5, 5.5) * pen.s());
            pen.p.add(Shape::ellipse_filled(center, radius, tint));
            pen.p.add(Shape::ellipse_stroke(center, radius, pen.stroke(c)));
        }
        "triangle" => solid(&[(10.0, 3.5), (17.0, 16.5), (3.0, 16.5)]),
        "diamond" => solid(&[(10.0, 3.0), (17.0, 10.0), (10.0, 17.0), (3.0, 10.0)]),
        "line" => pen.line(&[(4.0, 16.0), (16.0, 4.0)]),
        "arrow" => {
            pen.line(&[(4.0, 16.0), (14.5, 5.5)]);
            head((16.5, 3.5), (4.0, 16.0));
        }
        "star" => {
            let pts: Vec<(f32, f32)> = (0..10)
                .map(|i| {
                    let ang = -std::f32::consts::FRAC_PI_2 + i as f32 * std::f32::consts::PI / 5.0;
                    let rad = if i % 2 == 0 { 8.0 } else { 3.3 };
                    (10.0 + rad * ang.cos(), 10.8 + rad * ang.sin())
                })
                .collect();
            // Concave: the tinted core, then the outline.
            pen.fill(&pts.iter().skip(1).step_by(2).copied().collect::<Vec<_>>(), tint);
            pen.closed(&pts, c);
        }
        "heart" => {
            let pts: Vec<(f32, f32)> = (0..32)
                .map(|i| {
                    let s = i as f32 / 32.0 * std::f32::consts::TAU;
                    let x = 16.0 * s.sin().powi(3);
                    let y = 13.0 * s.cos() - 5.0 * (2.0 * s).cos() - 2.0 * (3.0 * s).cos() - (4.0 * s).cos();
                    (10.0 + x * 0.44, 9.6 - y * 0.44)
                })
                .collect();
            pen.closed(&pts, c);
        }
        "straightConnector" => ends(&[(4.0, 15.0), (16.0, 5.0)], false),
        "straightArrowConnector" => ends(&[(4.0, 15.0), (16.0, 5.0)], true),
        "elbowConnector" => ends(&[(4.0, 15.0), (10.0, 15.0), (10.0, 5.0), (16.0, 5.0)], false),
        "elbowArrowConnector" => ends(&[(4.0, 15.0), (10.0, 15.0), (10.0, 5.0), (16.0, 5.0)], true),
        "curvedConnector" => ends(&curve(), false),
        "curvedArrowConnector" => ends(&curve(), true),
        _ => return false,
    }
    true
}

/// Draw an Insert › Icons library icon into `r` in `c`, from the same path data the inserted
/// picture is rendered from (`letra_render::icon_lib`).
pub fn library(p: &Painter, r: Rect, icon: &letra_render::icon_lib::Icon, c: Color32) {
    use letra_render::icon_lib::{GRID, STROKE};
    let s = r.width().min(r.height()) / GRID;
    let o = r.center() - vec2(GRID / 2.0 * s, GRID / 2.0 * s);
    let stroke = Stroke::new(STROKE * s, c);
    for piece in icon.pieces() {
        let pts: Vec<Pos2> = piece.points.iter().map(|q| pos2(o.x + q[0] * s, o.y + q[1] * s)).collect();
        if piece.fill {
            p.add(Shape::convex_polygon(pts, c, Stroke::NONE));
        } else if piece.closed {
            p.add(Shape::closed_line(pts, stroke));
        } else {
            // Round caps, as on the inserted picture.
            for end in [pts.first(), pts.last()].into_iter().flatten() {
                p.circle_filled(*end, stroke.width / 2.0, c);
            }
            p.add(Shape::line(pts, stroke));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn has_default_highlight_bar(name: &str) -> bool {
        let ctx = egui::Context::default();
        let output = ctx.run_ui(egui::RawInput::default(), |ui| {
            paint(ui.painter(), Rect::from_min_size(Pos2::ZERO, egui::vec2(20.0, 20.0)), name, Color32::BLACK, Color32::BLACK);
        });
        let yellow = Color32::from_rgb(0xF2, 0xC8, 0x11);
        let found = output.shapes.iter().any(|shape| matches!(&shape.shape, Shape::Rect(rect) if rect.fill == yellow));
        output.drop_without_applying_deltas();
        found
    }

    /// #4087: No Color uses the pen-only icon, not the ordinary icon's yellow fallback bar.
    #[test]
    fn no_colour_highlight_icon_has_no_yellow_bar() {
        assert!(has_default_highlight_bar("highlight"));
        assert!(!has_default_highlight_bar("highlightNoColor"));
    }

    /// Every NDL icon the mapping names exists in the pinned nodalia-theme, so no button
    /// silently draws nothing.
    #[test]
    fn every_mapped_icon_exists_in_ndl() {
        let names = ndl_names_in_mapping();
        assert!(names.len() > 300, "the mapping was read: {}", names.len());
        for name in names {
            assert!(nodalia_theme::icons::get(name).is_some(), "NDL has no icon {name}");
        }
    }

    fn ndl_names_in_mapping() -> Vec<&'static str> {
        let me = include_str!("icons.rs");
        let start = me.find("pub fn ndl_name").unwrap_or(0);
        let end = me[start..].find("_ => return None").map_or(me.len(), |e| start + e);
        me[start..end].split("=> \"").skip(1).filter_map(|t| t.split('"').next()).collect()
    }

    /// #4412: the Shading buttons paint the last colour as their bar, so their icon has none of its own.
    #[test]
    fn the_shading_button_icon_has_no_bar_of_its_own() {
        assert!(has_default_highlight_bar("shading"));
        assert!(!has_default_highlight_bar("shadingNoBar"));
    }
}
