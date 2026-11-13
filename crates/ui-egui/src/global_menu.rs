//! The menus Letra hands to a desktop's global menu (the top bar of Nodalix and other Linux
//! shells): File, Edit, View, Insert, Format and Help, each item a command id run exactly as the
//! ribbon runs it. Pure and platform-free; the desktop app serves [`MenuBar`] over D-Bus through a
//! [`Backend`] (`apps/letra/src/linux_menu.rs`).
//!
//! Letra's own window has no menu bar (the ribbon is its menu), so nothing in the window changes
//! when the desktop shows these: they are a second way in, like the command search.

use serde_json::{Value, json};

use crate::WordApp;

/// Quit: not a command of the engine, so the menu names it itself and the app closes its window
/// (which asks about unsaved documents as the title bar's close does).
pub const QUIT: &str = "window.close";

#[derive(Clone, Debug, Default, PartialEq)]
pub struct MenuBar {
    pub menus: Vec<Menu>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Menu {
    pub title: String,
    pub items: Vec<Node>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Node {
    Item(Item),
    Separator,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Item {
    pub label: String,
    pub command: &'static str,
    pub params: Value,
    pub enabled: bool,
}

/// A desktop's global menu.
pub trait Backend {
    /// Show `bar`; called every frame, so an implementation returns early when nothing changed.
    fn sync(&mut self, bar: &MenuBar);
    /// The items chosen since the last call.
    fn drain(&mut self) -> Vec<Item>;
}

/// One row: a command id, the label to show when it is not the command's own, and its params.
type Row = (&'static str, Option<&'static str>, fn() -> Value);

fn none() -> Value {
    json!({})
}

const SEP: Row = ("", None, none);

/// The menus, in the order of a desktop app's menu bar.
fn layout() -> [(&'static str, Vec<Row>); 6] {
    [
        (
            "File",
            vec![
                ("file.new", Some("New"), none),
                ("file.open", Some("Open…"), none),
                SEP,
                ("file.save", Some("Save"), none),
                ("file.saveAs", Some("Save As…"), none),
                ("file.exportPdf", Some("Export as PDF…"), none),
                SEP,
                ("file.print", Some("Print…"), none),
                SEP,
                ("file.close", Some("Close"), none),
                (QUIT, Some("Quit"), none),
            ],
        ),
        (
            "Edit",
            vec![
                ("edit.undo", Some("Undo"), none),
                ("edit.redo", Some("Redo"), none),
                SEP,
                ("edit.cut", Some("Cut"), none),
                ("edit.copy", Some("Copy"), none),
                ("edit.paste", Some("Paste"), none),
                ("select.all", Some("Select All"), none),
                SEP,
                ("edit.find", Some("Find…"), none),
                ("edit.replace", Some("Replace…"), none),
                ("edit.goto", Some("Go To…"), none),
            ],
        ),
        (
            "View",
            vec![
                ("view.readMode", Some("Read Mode"), none),
                ("view.printLayout", Some("Print Layout"), none),
                ("view.webLayout", Some("Web Layout"), none),
                ("view.draft", Some("Draft"), none),
                ("view.focus", Some("Focus"), none),
                SEP,
                ("view.ruler", Some("Ruler"), none),
                SEP,
                ("view.zoomIn", Some("Zoom In"), none),
                ("view.zoomOut", Some("Zoom Out"), none),
                ("view.zoom100", Some("Actual Size"), none),
            ],
        ),
        (
            "Insert",
            vec![
                ("insert.pageBreak", Some("Page Break"), none),
                ("insert.table", Some("Table…"), || json!({"rows": 3, "cols": 3})),
                ("insert.picture", Some("Picture…"), none),
                ("insert.link", Some("Link…"), none),
                ("review.newComment", Some("Comment"), none),
            ],
        ),
        (
            "Format",
            vec![
                ("format.bold", Some("Bold"), none),
                ("format.italic", Some("Italic"), none),
                ("format.underline", Some("Underline"), none),
                SEP,
                ("format.font", Some("Font…"), none),
                SEP,
                ("review.spelling", Some("Spelling and Grammar"), none),
                ("review.wordCount", Some("Word Count"), none),
            ],
        ),
        (
            "Help",
            vec![
                ("file.options", Some("Preferences"), none),
                ("ui.dialog", Some("Keyboard Shortcuts"), || json!({"name": "customizeKeyboard"})),
                ("ui.checkForUpdates", Some("Check for Updates…"), none),
                SEP,
                ("ui.dialog", Some("About Letra"), || json!({"name": "about"})),
            ],
        ),
    ]
}

/// The menus as they stand: labels translated, each item enabled as its command is now.
pub fn bar(app: &WordApp) -> MenuBar {
    let menus = layout()
        .into_iter()
        .map(|(title, rows)| Menu {
            title: tl!(title).to_string(),
            items: rows
                .into_iter()
                .map(|(command, label, params)| {
                    if command.is_empty() {
                        return Node::Separator;
                    }
                    let spec = app.session.registry.get(command);
                    let label = label.or(spec.map(|s| s.label)).unwrap_or(command);
                    let enabled = spec.is_none_or(|s| (s.enabled)(&app.session).is_none());
                    Node::Item(Item { label: tl!(label).to_string(), command, params: params(), enabled })
                })
                .collect(),
        })
        .collect();
    MenuBar { menus }
}

impl WordApp {
    /// Bring the desktop's global menu up to date and run what was chosen in it.
    pub(crate) fn sync_global_menu(&mut self, ctx: &egui::Context) {
        let Some(mut menu) = self.global_menu.take() else { return };
        for item in menu.drain() {
            if !item.enabled {
                continue;
            }
            if item.command == QUIT {
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            } else if let Err(e) = self.run(item.command, item.params) {
                self.status(e);
            }
        }
        menu.sync(&bar(self));
        self.global_menu = Some(menu);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};

    fn app() -> WordApp {
        WordApp::new(letra_engine::Session::new(letra_doc::Document::new()), Default::default())
    }

    /// Every item names a command the app runs: an engine command or one of the app's own.
    #[test]
    fn every_item_is_a_command() {
        let a = app();
        for (title, rows) in layout() {
            for (command, _, _) in rows {
                if command.is_empty() || command == QUIT || command.starts_with("ui.") {
                    continue;
                }
                assert!(a.session.registry.get(command).is_some(), "{title} › {command} is not a command");
            }
        }
    }

    #[test]
    fn the_bar_has_the_six_menus_with_labels_and_state() {
        let bar = bar(&app());
        let titles: Vec<&str> = bar.menus.iter().map(|m| m.title.as_str()).collect();
        assert_eq!(titles, ["File", "Edit", "View", "Insert", "Format", "Help"]);
        let file = &bar.menus[0].items;
        assert!(matches!(&file[0], Node::Item(it) if it.label == "New" && it.enabled));
        assert!(bar.menus.iter().flat_map(|m| &m.items).any(|n| matches!(n, Node::Separator)));
        // Each item is enabled exactly when its command is.
        let a = app();
        for it in bar.menus.iter().flat_map(|m| &m.items).filter_map(|n| match n {
            Node::Item(it) => Some(it),
            Node::Separator => None,
        }) {
            let want = a.session.registry.get(it.command).is_none_or(|s| (s.enabled)(&a.session).is_none());
            assert_eq!(it.enabled, want, "{}", it.command);
        }
    }

    struct Fake {
        chosen: Vec<Item>,
        shown: Arc<Mutex<Vec<MenuBar>>>,
    }

    impl Backend for Fake {
        fn sync(&mut self, bar: &MenuBar) {
            self.shown.lock().unwrap().push(bar.clone());
        }
        fn drain(&mut self) -> Vec<Item> {
            std::mem::take(&mut self.chosen)
        }
    }

    /// A click in the desktop's menu runs the command as the ribbon would, and the menu is shown.
    #[test]
    fn a_chosen_item_runs_its_command() {
        let mut a = app();
        let shown = Arc::new(Mutex::new(Vec::new()));
        let item = Item { label: "About Letra".into(), command: "ui.dialog", params: json!({"name": "about"}), enabled: true };
        a.global_menu = Some(Box::new(Fake { chosen: vec![item], shown: shown.clone() }));
        a.sync_global_menu(&egui::Context::default());
        assert!(a.dialog.is_some(), "the About dialog opened");
        assert_eq!(shown.lock().unwrap().len(), 1);
    }
}
