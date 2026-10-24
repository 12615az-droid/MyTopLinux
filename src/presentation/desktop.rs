use crate::{
    application::{export, CollectionService, ExportOptions, MAX_IMPORT_BYTES, STRINGS},
    domain::{Appearance, Game, Query, SyncSettings},
    error::AppResult,
    infrastructure::SqliteStore,
    presentation::platform::{choose_file, clipboard, local_data_uri, read_limited},
};
use base64::{engine::general_purpose::STANDARD, Engine};
use gtk::prelude::*;
use serde::Deserialize;
use serde_json::{json, Value};
use std::{cell::RefCell, fs, path::PathBuf, rc::Rc};
use wry::{WebView, WebViewBuilder, WebViewBuilderExtUnix};

#[derive(Deserialize)]
struct Request {
    id: u64,
    command: String,
    #[serde(default)]
    data: Value,
}
struct App {
    store: SqliteStore,
    directory: PathBuf,
    window: gtk::Window,
    ui_test: bool,
}
impl App {
    fn appearance(&self) -> AppResult<Appearance> {
        Ok(self.store.setting::<Appearance>("appearance")?.sanitize())
    }
    fn assets(&self, games: &[Game]) -> Value {
        let mut assets = serde_json::Map::new();
        for game in games {
            if let Some(uri) = &game.cover_uri {
                if let Some(data) = local_data_uri(uri, 30 * 1024 * 1024) {
                    assets.insert(uri.clone(), json!(data));
                }
            }
        }
        Value::Object(assets)
    }
    fn collection(&mut self, data: Value) -> AppResult<Value> {
        let query: Query = serde_json::from_value(data).map_err(|e| e.to_string())?;
        let collection = CollectionService::new(&mut self.store).collection(&query)?;
        let assets = self.assets(&collection.games);
        Ok(
            json!({"games":collection.games,"total":collection.total,"average":collection.average,"genres":collection.genres,"assets":assets}),
        )
    }
    fn handle(&mut self, command: &str, data: Value) -> AppResult<Value> {
        match command {
            "testResult" if self.ui_test => {
                println!("MYTOP_UI_RESULT: {data}");
                if data["ok"] != true {
                    std::process::exit(1);
                }
                gtk::main_quit();
                Ok(Value::Null)
            }
            "testScreenshot" if self.ui_test => {
                if let Some(directory) = std::env::var_os("MYTOP_TEST_SCREENSHOTS") {
                    let name = data.as_str().unwrap_or("main");
                    if !["main", "appearance", "editor", "dark"].contains(&name) {
                        return Err("Invalid screenshot name".into());
                    }
                    let path = PathBuf::from(directory);
                    fs::create_dir_all(&path).map_err(|e| e.to_string())?;
                    let (width, height) = self.window.size();
                    let pixbuf = self
                        .window
                        .window()
                        .and_then(|w| w.pixbuf(0, 0, width, height))
                        .ok_or("Cannot capture window")?;
                    pixbuf
                        .savev(path.join(format!("{name}.png")), "png", &[])
                        .map_err(|e| e.to_string())?;
                }
                Ok(Value::Null)
            }
            "init" => {
                let appearance = self.appearance()?;
                let sync = self.store.setting::<SyncSettings>("sync")?;
                let font_data = appearance
                    .custom_font_path
                    .as_deref()
                    .and_then(|p| local_data_uri(p, 5 * 1024 * 1024));
                let locale = std::env::var("LANGUAGE")
                    .or_else(|_| std::env::var("LC_ALL"))
                    .or_else(|_| std::env::var("LANG"))
                    .unwrap_or("en".into());
                Ok(
                    json!({"appearance":appearance,"sync":sync,"locale":locale,"fontData":font_data}),
                )
            }
            "query" => self.collection(data),
            "save" => {
                let game = serde_json::from_value(data).map_err(|e| e.to_string())?;
                Ok(json!(CollectionService::new(&mut self.store).save(game)?))
            }
            "delete" => {
                CollectionService::new(&mut self.store)
                    .delete(data["id"].as_i64().ok_or("game_not_found")?)?;
                Ok(Value::Null)
            }
            "appearance" => {
                let mut settings: Appearance =
                    serde_json::from_value(data).map_err(|e| e.to_string())?;
                // Frontend may select only fonts imported into our private directory.
                let saved = self.appearance()?;
                settings.custom_font_path = saved.custom_font_path;
                settings.custom_font_name = saved.custom_font_name;
                let settings = settings.sanitize();
                self.store.set_setting("appearance", &settings)?;
                Ok(json!(settings))
            }
            "resetAppearance" => {
                let language = self.appearance()?.language_tag;
                let appearance = Appearance {
                    language_tag: language,
                    ..Default::default()
                };
                self.store.set_setting("appearance", &appearance)?;
                // Remove only the owned font, never a user-selected source file.
                let font = self.directory.join("custom-font.bin");
                if font.exists() {
                    fs::remove_file(font).map_err(|e| e.to_string())?;
                }
                Ok(json!(appearance))
            }
            "syncSave" => {
                let settings: SyncSettings =
                    serde_json::from_value(data).map_err(|e| e.to_string())?;
                let settings = settings.normalize();
                if (settings.enabled || !settings.server_url.is_empty()) && !settings.valid_url() {
                    return Err("sync_invalid_url".into());
                }
                self.store.set_setting("sync", &settings)?;
                Ok(json!(settings))
            }
            "syncNow" => Ok(json!(self
                .store
                .setting::<SyncSettings>("sync")?
                .synchronize())),
            "import" => {
                let Some(path) = choose_file(
                    &self.window,
                    gtk::FileChooserAction::Open,
                    "JSON",
                    &["*.json", "*.JSON"],
                    None,
                ) else {
                    return Ok(Value::Null);
                };
                let bytes = read_limited(&path, MAX_IMPORT_BYTES)?;
                let text = String::from_utf8(bytes).map_err(|_| "import_error")?;
                Ok(json!(CollectionService::new(&mut self.store).import(&text)?))
            }
            "export" | "copyExport" => {
                let options: ExportOptions =
                    serde_json::from_value(data["options"].clone()).map_err(|e| e.to_string())?;
                let query: Query =
                    serde_json::from_value(data["query"].clone()).map_err(|e| e.to_string())?;
                let language = data["language"].as_str().unwrap_or("en");
                let text = export(&self.store.games()?, &query, &options, language)?;
                if command == "copyExport" {
                    clipboard(&text);
                    return Ok(json!(true));
                }
                let name = if options.format == "JSON" {
                    "my-game-top.json"
                } else {
                    "my-game-top.txt"
                };
                let Some(path) = choose_file(
                    &self.window,
                    gtk::FileChooserAction::Save,
                    "MyTop",
                    &[],
                    Some(name),
                ) else {
                    return Ok(Value::Null);
                };
                fs::write(path, text).map_err(|e| e.to_string())?;
                Ok(json!(true))
            }
            "clipboard" => {
                clipboard(data.as_str().ok_or("export_error")?);
                Ok(Value::Null)
            }
            "cover" => {
                let Some(source) = choose_file(
                    &self.window,
                    gtk::FileChooserAction::Open,
                    "Image",
                    &["*.png", "*.jpg", "*.jpeg", "*.webp", "*.gif"],
                    None,
                ) else {
                    return Ok(Value::Null);
                };
                let bytes = read_limited(&source, 30 * 1024 * 1024)?;
                // Decode first: a file extension alone doesn't establish that this is an image.
                let loader = gtk::gdk_pixbuf::PixbufLoader::new();
                loader.write(&bytes).map_err(|e| e.to_string())?;
                loader.close().map_err(|e| e.to_string())?;
                if loader.pixbuf().is_none() {
                    return Err("cover_unavailable".into());
                }
                let extension = source
                    .extension()
                    .and_then(|e| e.to_str())
                    .unwrap_or("png")
                    .to_lowercase();
                let unique = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map_err(|e| e.to_string())?
                    .as_nanos();
                let path = self
                    .directory
                    .join("covers")
                    .join(format!("{unique}.{extension}"));
                fs::write(&path, bytes).map_err(|e| e.to_string())?;
                let uri = path.to_string_lossy().into_owned();
                Ok(json!({"uri":uri,"data":local_data_uri(&uri,30*1024*1024)}))
            }
            "font" => {
                let Some(path) = choose_file(
                    &self.window,
                    gtk::FileChooserAction::Open,
                    "TTF / OTF",
                    &["*.ttf", "*.otf", "*.TTF", "*.OTF"],
                    None,
                ) else {
                    return Ok(Value::Null);
                };
                let bytes = read_limited(&path, 5 * 1024 * 1024)?;
                if !(bytes.starts_with(&[0, 1, 0, 0])
                    || bytes.starts_with(b"OTTO")
                    || bytes.starts_with(b"true"))
                {
                    return Err("font_error".into());
                }
                let owned = self.directory.join("custom-font.bin");
                fs::write(&owned, &bytes).map_err(|e| e.to_string())?;
                let mut appearance = self.appearance()?;
                appearance.custom_font_path = Some(owned.to_string_lossy().into_owned());
                appearance.custom_font_name =
                    path.file_name().map(|n| n.to_string_lossy().into_owned());
                appearance.font = "CUSTOM".into();
                self.store.set_setting("appearance", &appearance)?;
                Ok(
                    json!({"appearance":appearance,"fontData":format!("data:font/ttf;base64,{}",STANDARD.encode(bytes))}),
                )
            }
            _ => Err("Unknown command".into()),
        }
    }
}
pub fn run() -> AppResult<()> {
    gtk::init().map_err(|e| e.to_string())?;
    gtk::glib::set_application_name("MyTop");
    gtk::glib::set_prgname(Some("mytop-linux"));
    let ui_test = std::env::args().any(|arg| arg == "--self-test-ui");
    let directory = if ui_test {
        let path = std::env::temp_dir().join(format!("mytop-ui-test-{}", std::process::id()));
        if path.exists() {
            return Err("UI test directory already exists".into());
        }
        path
    } else if let Some(path) = std::env::var_os("MYTOP_DATA_DIR") {
        PathBuf::from(path)
    } else {
        let base = std::env::var_os("XDG_DATA_HOME")
            .map(PathBuf::from)
            .filter(|p| p.is_absolute())
            .or_else(|| std::env::var_os("HOME").map(|p| PathBuf::from(p).join(".local/share")))
            .ok_or("No user data directory")?;
        base.join("mytop-linux")
    };
    fs::create_dir_all(directory.join("covers")).map_err(|e| e.to_string())?;
    let window = gtk::Window::new(gtk::WindowType::Toplevel);
    window.set_title("MyTop — Мой топ");
    window.set_default_size(540, 850);
    window.set_size_request(380, 500);
    let icon_loader = gtk::gdk_pixbuf::PixbufLoader::new();
    if icon_loader
        .write(include_bytes!("../../ui/icon.png"))
        .is_ok()
        && icon_loader.close().is_ok()
    {
        window.set_icon(icon_loader.pixbuf().as_ref());
    }
    window.connect_delete_event(|_, _| {
        gtk::main_quit();
        gtk::glib::Propagation::Proceed
    });
    let app = Rc::new(RefCell::new(App {
        ui_test,
        store: SqliteStore::open(&directory.join("mytop.sqlite3"))?,
        directory,
        window: window.clone(),
    }));
    let view: Rc<RefCell<Option<WebView>>> = Rc::new(RefCell::new(None));
    let callback_view = view.clone();
    let mut html = include_str!("../../ui/index.html")
        .replace("/* STYLES */", include_str!("../../ui/style.css"))
        .replace("/* TRANSLATIONS */", STRINGS)
        .replace("/* APPLICATION */", include_str!("../../ui/app.js"));
    if ui_test {
        html.push_str("<script>");
        html.push_str(include_str!("../../tests/ui-smoke.js"));
        html.push_str("</script>");
        gtk::glib::timeout_add_local_once(std::time::Duration::from_secs(45), || {
            eprintln!("UI test timed out");
            std::process::exit(1);
        });
    }
    let webview = WebViewBuilder::new()
        .with_html(html)
        .with_navigation_handler(|url| url == "about:blank")
        .with_new_window_req_handler(|_, _| wry::NewWindowResponse::Deny)
        .with_ipc_handler(move |request| {
            let Ok(request) = serde_json::from_str::<Request>(request.body()) else {
                return;
            };
            let app = app.clone();
            let view = callback_view.clone();
            // Run after the IPC callback returns; native file dialogs may run a nested GTK loop.
            gtk::glib::idle_add_local_once(move || {
                let result = app.borrow_mut().handle(&request.command, request.data);
                let response = match result {
                    Ok(value) => json!({"id":request.id,"ok":true,"data":value}),
                    Err(error) => json!({"id":request.id,"ok":false,"error":error}),
                };
                if let Some(view) = view.borrow().as_ref() {
                    let _ = view.evaluate_script(&format!("window.receive({response})"));
                }
            });
        })
        .build_gtk(&window)
        .map_err(|e| e.to_string())?;
    *view.borrow_mut() = Some(webview);
    window.show_all();
    gtk::main();
    Ok(())
}
