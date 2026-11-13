//! The Linux desktop's global menu: Letra's menus ([`letra_ui_egui::global_menu`]) exported over
//! D-Bus as a `com.canonical.dbusmenu` object and registered with the desktop's
//! `com.canonical.AppMenu.Registrar`, so a top bar that shows the focused app's menus (Nodalix's)
//! shows Letra's.
//!
//! A window on Wayland has no global id, so the registration names this process instead, and only
//! a registrar that pairs a menu with its window by process can show it: Nodalix's, whose process
//! also owns `space.unmade.GlobalMenu`. With any other registrar, or none, nothing is registered
//! and Letra works as before: its ribbon is its menu. Keys stay with the window.

use std::collections::HashMap;
use std::sync::{Arc, Mutex, PoisonError};

use letra_ui_egui::global_menu::{Backend, Item, MenuBar, Node};
use zbus::blocking::Connection;
use zbus::zvariant::{OwnedValue, StructureBuilder, Value};

const REGISTRAR: &str = "com.canonical.AppMenu.Registrar";
const REGISTRAR_PATH: &str = "/com/canonical/AppMenu/Registrar";
const NODALIX_MENU: &str = "space.unmade.GlobalMenu";
const MENU_PATH: &str = "/com/getnodalia/GlobalMenu";
const MENU_INTERFACE: &str = "com.canonical.dbusmenu";

/// One row of the exported menu.
#[derive(Clone, Debug, PartialEq)]
enum Row {
    Item(Box<Item>),
    Separator,
    Submenu { label: String, children: Vec<i32> },
}

/// The menus as the desktop reads them: rows by id (0 is the root, whose children are the
/// top-level menus).
#[derive(Default)]
struct Shared {
    rows: HashMap<i32, Row>,
    root: Vec<i32>,
    revision: u32,
    events: Vec<Item>,
}

impl Shared {
    /// Number the rows of `bar` depth first, from 1: the same structure always gets the same ids.
    fn rows_of(bar: &MenuBar) -> (HashMap<i32, Row>, Vec<i32>) {
        let (mut rows, mut next, mut root) = (HashMap::new(), 1i32, Vec::new());
        for menu in &bar.menus {
            let id = next;
            next = next.saturating_add(1);
            let mut children = Vec::new();
            for node in &menu.items {
                let child = next;
                next = next.saturating_add(1);
                rows.insert(
                    child,
                    match node {
                        Node::Item(it) => Row::Item(Box::new(it.clone())),
                        Node::Separator => Row::Separator,
                    },
                );
                children.push(child);
            }
            rows.insert(id, Row::Submenu { label: menu.title.clone(), children });
            root.push(id);
        }
        (rows, root)
    }

    fn children(&self, id: i32) -> &[i32] {
        match self.rows.get(&id) {
            _ if id == 0 => &self.root,
            Some(Row::Submenu { children, .. }) => children,
            _ => &[],
        }
    }

    /// `id`'s dbusmenu properties.
    fn properties(&self, id: i32) -> HashMap<String, OwnedValue> {
        let mut p = HashMap::new();
        let mut put = |k: &str, v: Value<'_>| {
            if let Ok(v) = v.try_to_owned() {
                p.insert(k.to_string(), v);
            }
        };
        match self.rows.get(&id) {
            None if id == 0 => put("children-display", Value::from("submenu")),
            None => {}
            Some(Row::Separator) => put("type", Value::from("separator")),
            Some(Row::Submenu { label, .. }) => {
                put("label", Value::from(mnemonic_free(label)));
                put("children-display", Value::from("submenu"));
            }
            Some(Row::Item(item)) => {
                put("label", Value::from(mnemonic_free(&item.label)));
                put("enabled", Value::from(item.enabled));
            }
        }
        p
    }

    /// `id` and `depth` levels of its children (`-1`: all), the reply to `GetLayout`.
    fn layout_item(&self, id: i32, depth: i32) -> LayoutItem {
        let children = if depth == 0 {
            Vec::new()
        } else {
            self.children(id).iter().filter_map(|&c| self.layout(c, depth.saturating_sub(1)).try_to_owned().ok()).collect()
        };
        LayoutItem { id, properties: self.properties(id), children }
    }

    /// [`Shared::layout_item`] as a variant's value: how dbusmenu nests the children.
    fn layout(&self, id: i32, depth: i32) -> Value<'static> {
        let children: Vec<Value<'static>> =
            if depth == 0 { Vec::new() } else { self.children(id).iter().map(|&c| self.layout(c, depth.saturating_sub(1))).collect() };
        let children = zbus::zvariant::Array::from(children);
        let built = StructureBuilder::new().add_field(id).add_field(self.properties(id)).append_field(Value::Array(children)).build();
        built.map(Value::Structure).unwrap_or_else(|_| Value::from(0))
    }
}

/// A label with its underscores doubled: dbusmenu reads a single one as a mnemonic.
fn mnemonic_free(label: &str) -> String {
    label.replace('_', "__")
}

/// One item of the menu tree as dbusmenu sends it: `(ia{sv}av)`, each child a variant holding the
/// same structure.
#[derive(Debug, serde::Serialize, zbus::zvariant::Type)]
struct LayoutItem {
    id: i32,
    properties: HashMap<String, OwnedValue>,
    children: Vec<OwnedValue>,
}

/// The `com.canonical.dbusmenu` object.
struct MenuObject {
    shared: Arc<Mutex<Shared>>,
    ctx: egui::Context,
}

impl MenuObject {
    fn lock(&self) -> std::sync::MutexGuard<'_, Shared> {
        self.shared.lock().unwrap_or_else(PoisonError::into_inner)
    }

    fn chosen(&self, id: i32, kind: &str) {
        if kind != "clicked" {
            return;
        }
        let mut s = self.lock();
        if let Some(Row::Item(item)) = s.rows.get(&id)
            && item.enabled
        {
            let item = (**item).clone();
            s.events.push(item);
            drop(s);
            self.ctx.request_repaint();
        }
    }
}

#[zbus::interface(name = "com.canonical.dbusmenu")]
impl MenuObject {
    #[zbus(property)]
    fn version(&self) -> u32 {
        3
    }
    #[zbus(property)]
    fn text_direction(&self) -> String {
        "ltr".into()
    }
    #[zbus(property)]
    fn status(&self) -> String {
        "normal".into()
    }
    #[zbus(property)]
    fn icon_theme_path(&self) -> Vec<String> {
        Vec::new()
    }

    fn get_layout(&self, parent_id: i32, recursion_depth: i32, _property_names: Vec<String>) -> (u32, LayoutItem) {
        let s = self.lock();
        (s.revision, s.layout_item(parent_id, recursion_depth))
    }

    fn get_group_properties(&self, ids: Vec<i32>, _property_names: Vec<String>) -> Vec<(i32, HashMap<String, OwnedValue>)> {
        let s = self.lock();
        // An empty list asks for every item (the dbusmenu contract).
        let ids = if ids.is_empty() { s.rows.keys().copied().collect() } else { ids };
        ids.into_iter().filter(|id| s.rows.contains_key(id)).map(|id| (id, s.properties(id))).collect()
    }

    fn get_property(&self, id: i32, name: String) -> OwnedValue {
        self.lock().properties(id).remove(&name).unwrap_or_else(|| OwnedValue::from(0i32))
    }

    fn event(&self, id: i32, event_id: String, _data: OwnedValue, _timestamp: u32) {
        self.chosen(id, &event_id);
    }

    fn event_group(&self, events: Vec<(i32, String, OwnedValue, u32)>) -> Vec<i32> {
        for (id, kind, _, _) in events {
            self.chosen(id, &kind);
        }
        Vec::new()
    }

    fn about_to_show(&self, _id: i32) -> bool {
        false
    }

    fn about_to_show_group(&self, ids: Vec<i32>) -> (Vec<i32>, Vec<i32>) {
        let s = self.lock();
        let unknown = ids.into_iter().filter(|id| *id != 0 && !s.rows.contains_key(id)).collect();
        (Vec::new(), unknown)
    }
}

/// The global menu's [`Backend`].
struct LinuxMenu {
    conn: Connection,
    shared: Arc<Mutex<Shared>>,
}

impl LinuxMenu {
    fn lock(&self) -> std::sync::MutexGuard<'_, Shared> {
        self.shared.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

impl Backend for LinuxMenu {
    fn sync(&mut self, bar: &MenuBar) {
        let (rows, root) = Shared::rows_of(bar);
        let revision = {
            let mut s = self.lock();
            if s.rows == rows && s.root == root {
                return;
            }
            s.rows = rows;
            s.root = root;
            s.revision = s.revision.wrapping_add(1);
            s.revision
        };
        // The desktop reads the menus again when told they changed; a failure leaves it on the
        // last ones it read.
        let _ = self.conn.emit_signal(None::<&str>, MENU_PATH, MENU_INTERFACE, "LayoutUpdated", &(revision, 0i32));
    }

    fn drain(&mut self) -> Vec<Item> {
        std::mem::take(&mut self.lock().events)
    }
}

/// Whether the registrar on the bus is Nodalix's, the one registrar that finds a Wayland
/// window's menu by its process.
fn nodalix_registrar(conn: &Connection) -> bool {
    let Ok(dbus) = zbus::blocking::fdo::DBusProxy::new(conn) else { return false };
    let owner = |name: &str| zbus::names::BusName::try_from(name).ok().and_then(|n| dbus.get_name_owner(n).ok()).map(|o| o.to_string());
    matches!((owner(REGISTRAR), owner(NODALIX_MENU)), (Some(r), Some(n)) if r == n)
}

/// Hand the menu to the registrar.
fn register(conn: &Connection) {
    let path = zbus::zvariant::ObjectPath::from_static_str_unchecked(MENU_PATH);
    let accepted = nodalix_registrar(conn)
        && conn.call_method(Some(REGISTRAR), REGISTRAR_PATH, Some(REGISTRAR), "RegisterWindow", &(std::process::id(), path)).is_ok();
    log::info!("global menu: {}", if accepted { "shown in the desktop's top bar" } else { "no registrar that shows it" });
}

/// Export the menus and register them with the desktop's registrar, now and whenever one
/// appears. `None` off Wayland or without a session bus.
pub fn install(ctx: &egui::Context, app: &letra_ui_egui::WordApp, wayland: bool) -> Option<Box<dyn Backend>> {
    if !wayland {
        return None;
    }
    let shared = Arc::new(Mutex::new(Shared { revision: 1, ..Shared::default() }));
    {
        let (rows, root) = Shared::rows_of(&letra_ui_egui::global_menu::bar(app));
        let mut s = shared.lock().unwrap_or_else(PoisonError::into_inner);
        s.rows = rows;
        s.root = root;
    }
    let object = MenuObject { shared: shared.clone(), ctx: ctx.clone() };
    let conn = match zbus::blocking::connection::Builder::session().and_then(|b| b.serve_at(MENU_PATH, object)).and_then(|b| b.build()) {
        Ok(conn) => conn,
        Err(e) => {
            log::info!("global menu: no session bus ({e})");
            return None;
        }
    };
    register(&conn);
    // The registrar may start after Letra, or restart with the shell: register whenever it
    // appears.
    let watch_conn = conn.clone();
    let spawned = std::thread::Builder::new().name("global-menu".into()).spawn(move || {
        let Ok(dbus) = zbus::blocking::fdo::DBusProxy::new(&watch_conn) else { return };
        let Ok(changes) = dbus.receive_name_owner_changed() else { return };
        for change in changes {
            let Ok(args) = change.args() else { continue };
            let name = args.name().as_str();
            if (name == REGISTRAR || name == NODALIX_MENU) && args.new_owner().is_some() {
                register(&watch_conn);
            }
        }
    });
    if let Err(e) = spawned {
        log::warn!("global menu: couldn't watch the registrar ({e})");
    }
    Some(Box::new(LinuxMenu { conn, shared }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use letra_ui_egui::global_menu::Menu;

    fn item(label: &str, enabled: bool) -> Node {
        Node::Item(Item { label: label.into(), command: "file.new", params: serde_json::json!({}), enabled })
    }

    fn bar() -> MenuBar {
        MenuBar { menus: vec![Menu { title: "File".into(), items: vec![item("New_Doc", true), Node::Separator, item("Soon", false)] }] }
    }

    #[test]
    fn rows_are_numbered_depth_first_with_the_menus_at_the_root() {
        let (rows, root) = Shared::rows_of(&bar());
        assert_eq!(root, vec![1]);
        assert_eq!(rows.len(), 4);
        let s = Shared { rows, root, ..Default::default() };
        assert_eq!(s.children(0), &[1]);
        assert_eq!(s.children(1), &[2, 3, 4]);
        // Mnemonics are escaped; a disabled item says so; separators are typed.
        let p = s.properties(2);
        assert_eq!(p.get("label").and_then(|v| <&str>::try_from(v).ok()), Some("New__Doc"));
        assert_eq!(s.properties(4).get("enabled").and_then(|v| bool::try_from(v).ok()), Some(false));
        assert_eq!(s.properties(3).get("type").and_then(|v| <&str>::try_from(v).ok()), Some("separator"));
        // The layout carries the whole tree, typed as dbusmenu reads it.
        use zbus::zvariant::Type as _;
        assert_eq!(LayoutItem::SIGNATURE.to_string(), "(ia{sv}av)");
        let root = s.layout_item(0, -1);
        assert_eq!(root.children.len(), 1);
        assert!(s.layout_item(0, 0).children.is_empty());
    }

    #[test]
    fn a_click_reports_the_item_and_a_disabled_one_nothing() {
        let (rows, root) = Shared::rows_of(&bar());
        let object = MenuObject { shared: Arc::new(Mutex::new(Shared { rows, root, ..Default::default() })), ctx: egui::Context::default() };
        object.chosen(2, "clicked");
        object.chosen(4, "clicked");
        object.chosen(2, "hovered");
        let events = std::mem::take(&mut object.lock().events);
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].label, "New_Doc");
    }
}
