//! The menu bar of the window: Help with About and Report an issue (and, on macOS, the menus every
//! program of that system has). What is chosen is told to the screen, which does the work.

use serde::Deserialize;
#[cfg(target_os = "macos")]
use tauri::menu::PredefinedMenuItem;
use tauri::{
    AppHandle, Emitter, Manager, Wry,
    menu::{Menu, MenuEvent, MenuItem, Submenu, SubmenuBuilder},
};

/// The event sent to the screen when a menu entry of ours is chosen; its payload says which.
pub const MENU_EVENT: &str = "menu-action";
const ABOUT_ID: &str = "help-about";
const REPORT_ID: &str = "help-report-issue";
const DOCUMENTATION_ID: &str = "help-documentation";
#[cfg(target_os = "macos")]
const APP_ABOUT_ID: &str = "app-about";

/// The words of the menu in the language of the screen.
#[derive(Debug, Deserialize)]
pub struct MenuLabels {
    pub help: String,
    pub about: String,
    pub report_issue: String,
    pub documentation: String,
}

/// The entries whose words change with the language.
pub struct MenuHandles {
    help: Submenu<Wry>,
    about: MenuItem<Wry>,
    report: MenuItem<Wry>,
    documentation: MenuItem<Wry>,
    #[cfg(target_os = "macos")]
    app_about: MenuItem<Wry>,
}

impl MenuHandles {
    pub fn set_text(&self, labels: &MenuLabels) -> tauri::Result<()> {
        self.help.set_text(&labels.help)?;
        self.about.set_text(&labels.about)?;
        self.report.set_text(&labels.report_issue)?;
        self.documentation.set_text(&labels.documentation)?;
        #[cfg(target_os = "macos")]
        self.app_about.set_text(&labels.about)?;
        Ok(())
    }
}

fn item(app: &AppHandle, id: &str, text: &str) -> tauri::Result<MenuItem<Wry>> {
    MenuItem::with_id(app, id, text, true, None::<&str>)
}

/// Puts the menu bar on the window (the menu bar of the screen on macOS).
pub fn install(app: &AppHandle) -> tauri::Result<()> {
    let about = item(app, ABOUT_ID, "About STARY")?;
    let report = item(app, REPORT_ID, "Report an issue")?;
    let documentation = item(app, DOCUMENTATION_ID, "Documentation")?;
    let help = SubmenuBuilder::new(app, "Help")
        .item(&about)
        .item(&report)
        .item(&documentation)
        .build()?;
    let menu = Menu::new(app)?;

    #[cfg(target_os = "macos")]
    let app_about = {
        let app_about = item(app, APP_ABOUT_ID, "About STARY")?;
        let name = SubmenuBuilder::new(app, "STARY")
            .item(&app_about)
            .separator()
            .item(&PredefinedMenuItem::services(app, None)?)
            .separator()
            .item(&PredefinedMenuItem::hide(app, None)?)
            .item(&PredefinedMenuItem::hide_others(app, None)?)
            .item(&PredefinedMenuItem::show_all(app, None)?)
            .separator()
            .item(&PredefinedMenuItem::quit(app, None)?)
            .build()?;
        let edit = SubmenuBuilder::new(app, "Edit")
            .item(&PredefinedMenuItem::undo(app, None)?)
            .item(&PredefinedMenuItem::redo(app, None)?)
            .separator()
            .item(&PredefinedMenuItem::cut(app, None)?)
            .item(&PredefinedMenuItem::copy(app, None)?)
            .item(&PredefinedMenuItem::paste(app, None)?)
            .item(&PredefinedMenuItem::select_all(app, None)?)
            .build()?;
        let window = SubmenuBuilder::new(app, "Window")
            .item(&PredefinedMenuItem::minimize(app, None)?)
            .item(&PredefinedMenuItem::maximize(app, None)?)
            .separator()
            .item(&PredefinedMenuItem::close_window(app, None)?)
            .build()?;
        menu.append(&name)?;
        menu.append(&edit)?;
        menu.append(&window)?;
        app_about
    };

    menu.append(&help)?;
    app.set_menu(menu)?;
    app.manage(MenuHandles {
        help,
        about,
        report,
        documentation,
        #[cfg(target_os = "macos")]
        app_about,
    });
    Ok(())
}

/// Tells the screen what was chosen in the menu.
pub fn on_event(app: &AppHandle, event: MenuEvent) {
    let action = match event.id().as_ref() {
        ABOUT_ID => "about",
        #[cfg(target_os = "macos")]
        APP_ABOUT_ID => "about",
        REPORT_ID => "report_issue",
        DOCUMENTATION_ID => "documentation",
        _ => return,
    };
    let _ = app.emit(MENU_EVENT, action);
}
