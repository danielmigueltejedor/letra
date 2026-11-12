//! Software updates: Help ▸ Check for Updates, the check at start, and their windows.
//!
//! The flow and the windows are `nodalia-update`'s, shared with the other Nodalia apps; this file
//! is what is this app's own: when an update may close it.

use crate::WordApp;

#[cfg(not(target_arch = "wasm32"))]
use nodalia_update::egui::{Action, Updater};

pub(crate) struct Runtime {
    #[cfg(not(target_arch = "wasm32"))]
    updater: Updater,
    /// The desktop app turns the check at start on; tests and the web build leave it off.
    #[cfg(not(target_arch = "wasm32"))]
    startup_check: bool,
    #[cfg(not(target_arch = "wasm32"))]
    check_requested: bool,
    /// Why updates are off in this process, which Help ▸ Check for Updates then says.
    #[cfg(not(target_arch = "wasm32"))]
    disabled: Option<String>,
    /// The context of the last frame, to wake the UI when a check is queued from a menu callback.
    #[cfg(not(target_arch = "wasm32"))]
    repaint: Option<egui::Context>,
    /// The web build has no updater; its window only says so.
    #[cfg(target_arch = "wasm32")]
    pub(crate) manual_window_open: bool,
    #[cfg(target_arch = "wasm32")]
    pub(crate) status_message: Option<String>,
}

impl Default for Runtime {
    fn default() -> Self {
        Self {
            #[cfg(not(target_arch = "wasm32"))]
            updater: Updater::new(crate::UPDATE_APP).with_accent([0x3f, 0x5f, 0xe0]),
            #[cfg(not(target_arch = "wasm32"))]
            startup_check: false,
            #[cfg(not(target_arch = "wasm32"))]
            check_requested: false,
            #[cfg(not(target_arch = "wasm32"))]
            disabled: None,
            #[cfg(not(target_arch = "wasm32"))]
            repaint: None,
            #[cfg(target_arch = "wasm32")]
            manual_window_open: false,
            #[cfg(target_arch = "wasm32")]
            status_message: None,
        }
    }
}

impl WordApp {
    /// Whether the app looks for a newer release when it starts (off until the desktop app turns it on).
    #[cfg(not(target_arch = "wasm32"))]
    pub fn set_startup_update_check(&mut self, on: bool) {
        self.updates_rt.startup_check = on;
    }

    /// Turns updates off for the rest of this process: none is looked for at start, and Help ▸
    /// Check for Updates says why. For a process that still carries another program's AppImage
    /// variables (#5653); `nodalia-update` also refuses to replace an AppImage this process does
    /// not run from.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn disable_self_update(&mut self) {
        let rt = &mut self.updates_rt;
        rt.startup_check = false;
        rt.disabled = Some(
            tl!("Updates are off in this window: WordCraft was started with another program's AppImage settings. Start WordCraft from its own menu entry to update it.")
                .to_string(),
        );
    }
}

pub(crate) fn check(app: &mut WordApp) {
    #[cfg(not(target_arch = "wasm32"))]
    {
        app.updates_rt.check_requested = true;
        if let Some(ctx) = &app.updates_rt.repaint {
            ctx.request_repaint();
        }
    }
    #[cfg(target_arch = "wasm32")]
    {
        app.updates_rt.manual_window_open = true;
        app.updates_rt.status_message = Some(tl!("Update check is not supported in the web browser.").to_string());
    }
}

pub(crate) fn tick(app: &mut WordApp, ctx: &egui::Context) {
    #[cfg(not(target_arch = "wasm32"))]
    {
        let rt = &mut app.updates_rt;
        rt.repaint = Some(ctx.clone());
        if std::mem::take(&mut rt.check_requested) {
            match &rt.disabled {
                Some(why) => rt.updater.set_status(nodalia_update::egui::Status::CheckFailed(why.clone())),
                None => rt.updater.check_now(ctx),
            }
        } else if rt.startup_check {
            rt.updater.check_at_start(ctx);
        }
        let action = rt.updater.poll();
        act(app, ctx, action);
    }
    #[cfg(target_arch = "wasm32")]
    let _ = (app, ctx);
}

/// Carries out what the update windows asked for.
#[cfg(not(target_arch = "wasm32"))]
fn act(app: &mut WordApp, ctx: &egui::Context, action: Action) {
    match action {
        Action::OpenUrl(url) => ctx.open_url(egui::OpenUrl::new_tab(url)),
        Action::Exit => {
            // The new version is starting, or a helper waits for this process to end. The whole
            // app quits, every window included, which saves the preferences; the install only ran
            // once nothing was unsaved. Exit anyway if the app has not ended soon.
            let _ = app.ask_before_quit();
            std::thread::spawn(|| {
                std::thread::sleep(std::time::Duration::from_secs(5));
                std::process::exit(0);
            });
        }
        Action::None => {}
    }
}

pub(crate) fn show(app: &mut WordApp, ctx: &egui::Context) {
    #[cfg(not(target_arch = "wasm32"))]
    {
        // An installation waits until no work is unsaved.
        let clean = !(app.session.dirty || app.save_unconfirmed() || app.printing.busy() || crate::panes::has_comment_drafts(app));
        let language = nodalia_update::Language::from_code(crate::i18n::current().code());
        let action = app.updates_rt.updater.show(ctx, language, clean);
        act(app, ctx, action);
    }

    #[cfg(target_arch = "wasm32")]
    if app.updates_rt.manual_window_open {
        let mut open = true;
        egui::Window::new(tl!("Check for Updates"))
            .open(&mut open)
            .resizable(false)
            .collapsible(false)
            .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
            .show(ctx, |ui| {
                crate::rtl::root(ui, |ui| {
                    if let Some(msg) = &app.updates_rt.status_message {
                        ui.label(msg);
                    }
                })
            });
        if !open {
            app.updates_rt.manual_window_open = false;
            app.updates_rt.status_message = None;
        }
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use nodalia_update::egui::{Status, Updater};

    fn app() -> crate::WordApp {
        let mut app = crate::WordApp::new(wordcraft_engine::Session::new(wordcraft_doc::Document::new()), crate::Services::default());
        app.updates_rt.updater = Updater::new(crate::UPDATE_APP).with_stamp(None);
        app
    }

    fn frame(app: &mut crate::WordApp, ctx: &egui::Context, events: Vec<egui::Event>) {
        let mut out = ctx.run_ui(egui::RawInput { events, ..Default::default() }, |ui| {
            super::tick(app, ui.ctx());
            super::show(app, ui.ctx());
        });
        out.textures_delta.clear();
    }

    #[test]
    fn the_releases_are_this_apps_own() {
        assert_eq!(crate::UPDATE_APP.repo, "danielmigueltejedor/letra");
        assert_eq!(crate::UPDATE_APP.version, env!("CARGO_PKG_VERSION"));
    }

    /// With updates off, no check is made at start, and Help ▸ Check for Updates ends in the
    /// reason without looking for a release (#5653).
    #[test]
    fn with_the_self_update_off_a_check_finds_nothing_and_says_why() {
        let mut app = app();
        app.set_startup_update_check(true);
        app.disable_self_update();
        assert!(!app.updates_rt.startup_check, "no check at start");
        let ctx = egui::Context::default();
        super::check(&mut app);
        frame(&mut app, &ctx, vec![]);
        let status = app.updates_rt.updater.status();
        assert!(matches!(status, Status::CheckFailed(why) if why.contains("another program's AppImage")), "{status:?}");
    }

    /// Help ▸ Check for Updates runs while the ribbon draws, after the frame's `tick`: the window
    /// shows the check from the next frame on, and Esc closes it.
    #[test]
    fn a_check_asked_for_while_drawing_opens_its_window() {
        let mut app = app();
        app.disable_self_update();
        let ctx = egui::Context::default();
        let mut out = ctx.run_ui(egui::RawInput::default(), |ui| {
            super::tick(&mut app, ui.ctx());
            super::check(&mut app);
            super::show(&mut app, ui.ctx());
        });
        out.textures_delta.clear();
        frame(&mut app, &ctx, vec![]);
        assert!(app.updates_rt.updater.is_open());
        let escape = egui::Event::Key { key: egui::Key::Escape, physical_key: None, pressed: true, repeat: false, modifiers: egui::Modifiers::NONE };
        frame(&mut app, &ctx, vec![escape]);
        assert!(!app.updates_rt.updater.is_open(), "Esc closes the window");
        assert_eq!(app.updates_rt.updater.status(), &Status::Idle);
    }
}
