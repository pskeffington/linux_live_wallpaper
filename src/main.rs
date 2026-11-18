use anyhow::{bail, Context, Result};
use clap::Parser;
use gtk::gio::prelude::*;
use gtk::prelude::*;
use gtk::{Application, ApplicationWindow};
use gtk4_layer_shell::{Edge, KeyboardMode, Layer, LayerShell};
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use webkit::prelude::*;

#[derive(Clone, Debug)]
struct SourceSettings {
    zoom: f64,
    opacity: f64,
    reload_seconds: u32,
    custom_css: Option<String>,
    custom_js: Option<String>,
    user_agent: Option<String>,
    monitor_connector: Option<String>,
}

#[derive(Debug, Parser)]
#[command(name = "linux-live-wallpaper", version, about)]
struct Args {
    /// Website(s) to show as the live wallpaper. Repeat the option to add sources.
    #[arg(long = "url", value_name = "URL", num_args = 1..)]
    urls: Vec<String>,

    /// Add URL(s) to the persistent source list.
    #[arg(long = "add", value_name = "URL", num_args = 1..)]
    add: Vec<String>,

    /// Remove URL(s) from the persistent source list.
    #[arg(long = "remove", value_name = "URL", num_args = 1..)]
    remove: Vec<String>,

    /// Print the persistent source list and exit.
    #[arg(long)]
    list: bool,

    /// Use an alternate source-list file.
    #[arg(long, value_name = "PATH")]
    config: Option<PathBuf>,

    /// Print available displays with their zero-based GDK indexes and exit.
    #[arg(long)]
    list_monitors: bool,

    /// Display index to use (zero-based, ordered by GDK).
    #[arg(long, value_name = "INDEX")]
    monitor: Option<u32>,

    /// Seconds between automatic switches. Zero disables rotation.
    #[arg(long, default_value_t = 0, value_parser = clap::value_parser!(u32).range(0..))]
    rotate_seconds: u32,

    /// Reload the current page every N seconds. Zero disables automatic reload.
    #[arg(long, default_value_t = 0, value_parser = clap::value_parser!(u32).range(0..))]
    reload_seconds: u32,

    /// Inject CSS from a file into each loaded page.
    #[arg(long, value_name = "PATH")]
    css_file: Option<PathBuf>,

    /// Inject JavaScript from a file into each loaded page at document start.
    #[arg(long, value_name = "PATH")]
    js_file: Option<PathBuf>,

    /// Invert page colors while retaining the original appearance of images and video.
    #[arg(long)]
    invert_colors: bool,

    /// Opacity for the complete wallpaper surface, from 0.0 (transparent) to 1.0 (opaque).
    #[arg(long, default_value = "1.0", value_name = "0.0..1.0")]
    opacity: String,

    /// Page zoom factor from 0.25 to 5.0.
    #[arg(long, default_value = "1.0", value_name = "0.25..5.0")]
    zoom: String,

    /// Trust self-signed TLS certificates for this host only (repeatable; host excludes port).
    #[arg(long = "allow-self-signed", value_name = "HOST", action = clap::ArgAction::Append)]
    allow_self_signed_hosts: Vec<String>,

    /// Custom HTTP User-Agent for loaded web pages.
    #[arg(long, value_name = "STRING")]
    user_agent: Option<String>,

    /// Request a transparent web and GTK window background (compositor support varies).
    #[arg(long)]
    transparent_background: bool,

    /// Hide the wallpaper while a Linux battery reports discharging.
    #[arg(long)]
    deactivate_on_battery: bool,

    /// Run without the native controller window (intended for session autostart).
    #[arg(long)]
    background: bool,
}

fn main() -> Result<()> {
    let args = Args::parse();
    if args.list_monitors {
        let app = Application::builder()
            .application_id("io.github.pskeffington.LinuxLiveWallpaper")
            .build();
        app.connect_activate(|app| {
            match gtk::gdk::Display::default() {
                Some(display) => {
                    let monitors = display.monitors();
                    for index in 0..monitors.n_items() {
                        if let Some(monitor) = monitors
                            .item(index)
                            .and_then(|item| item.downcast::<gtk::gdk::Monitor>().ok())
                        {
                            let connector = monitor.connector().unwrap_or_default();
                            let model = monitor.model().unwrap_or_default();
                            let geometry = monitor.geometry();
                            println!(
                                "{index}: {} {} ({}x{} at {},{})",
                                connector,
                                model,
                                geometry.width(),
                                geometry.height(),
                                geometry.x(),
                                geometry.y()
                            );
                        }
                    }
                }
                None => eprintln!("No GDK display is available."),
            }
            app.quit();
        });
        app.run();
        return Ok(());
    }

    let config_path = args.config.clone().map(Ok).unwrap_or_else(default_config_path)?;
    let mut saved_sources = load_sources(&config_path)?;

    if args.list {
        for source in &saved_sources {
            println!("{source}");
        }
        return Ok(());
    }

    for source in &args.add {
        if !is_supported_url(source) {
            bail!("unsupported URL scheme in --add: {source}");
        }
        if !saved_sources.contains(source) {
            saved_sources.push(source.clone());
        }
    }
    for source in &args.remove {
        saved_sources.retain(|saved| saved != source);
    }

    if !args.add.is_empty() || !args.remove.is_empty() {
        save_sources(&config_path, &saved_sources)?;
    }

    // Source management commands do not start another wallpaper instance unless
    // the caller also supplied an explicit --url.
    if args.urls.is_empty() && (!args.add.is_empty() || !args.remove.is_empty()) {
        return Ok(());
    }

    let restore_saved_source = args.urls.is_empty();
    let urls = if args.urls.is_empty() {
        if saved_sources.is_empty() {
            bail!("No wallpaper sources configured. Add one with --add URL or start with --url URL.");
        }
        saved_sources
    } else {
        args.urls
    };

    if urls.iter().any(|url| !is_supported_url(url)) {
        bail!("only http://, https:// and file:// wallpaper URLs are supported");
    }

    let opacity = parse_opacity(&args.opacity)?;
    let zoom = parse_zoom(&args.zoom)?;
    let allow_self_signed_hosts: Vec<String> = args
        .allow_self_signed_hosts
        .iter()
        .map(|host| {
            normalize_tls_host(host)
                .with_context(|| format!("invalid host for --allow-self-signed: {host}"))
        })
        .collect::<Result<_>>()?;
    let user_agent = args.user_agent;
    let custom_css = read_optional_file(args.css_file.as_ref())?;
    let custom_js = read_optional_file(args.js_file.as_ref())?;
    let monitor = args.monitor;
    let rotate_seconds = args.rotate_seconds;
    let reload_seconds = args.reload_seconds;
    let invert_colors = args.invert_colors;
    let transparent_background = args.transparent_background;
    let deactivate_on_battery = args.deactivate_on_battery;
    let background = args.background;
    let startup_error = Rc::new(RefCell::new(None::<String>));

    let app = Application::builder()
        .application_id("io.github.pskeffington.LinuxLiveWallpaper")
        .build();
    let config_file = config_path.clone();
    let activation_error = startup_error.clone();
    app.connect_activate(move |app| {
        if let Err(error) = build_wallpaper(
            app,
            &urls,
            config_file.clone(),
            monitor,
            rotate_seconds,
            reload_seconds,
            opacity,
            zoom,
            user_agent.as_deref(),
            &allow_self_signed_hosts,
            invert_colors,
            transparent_background,
            deactivate_on_battery,
            background,
            restore_saved_source,
            &custom_css,
            &custom_js,
        ) {
            let message = format!("Could not create wallpaper surface: {error:#}");
            eprintln!("{message}");
            *activation_error.borrow_mut() = Some(message);
            app.quit();
        }
    });
    app.run();
    if let Some(error) = startup_error.borrow_mut().take() {
        bail!(error);
    }
    Ok(())
}

fn monitor_at(display: &gtk::gdk::Display, index: u32) -> Option<gtk::gdk::Monitor> {
    display
        .monitors()
        .item(index)
        .and_then(|item| item.downcast::<gtk::gdk::Monitor>().ok())
}

fn monitor_dimensions(monitor: &gtk::gdk::Monitor) -> (u32, u32) {
    let geometry = monitor.geometry();
    let scale_factor = monitor.scale_factor().max(1) as u32;
    (
        (geometry.width().max(1) as u32).saturating_mul(scale_factor),
        (geometry.height().max(1) as u32).saturating_mul(scale_factor),
    )
}

fn find_monitor_by_connector(
    display: &gtk::gdk::Display,
    connector: &str,
) -> Option<gtk::gdk::Monitor> {
    let monitors = display.monitors();
    (0..monitors.n_items()).find_map(|index| {
        let monitor = monitors
            .item(index)
            .and_then(|item| item.downcast::<gtk::gdk::Monitor>().ok())?;
        (monitor.connector().as_deref() == Some(connector)).then_some(monitor)
    })
}

fn default_config_path() -> Result<PathBuf> {
    if let Some(path) = std::env::var_os("XDG_CONFIG_HOME") {
        return Ok(PathBuf::from(path)
            .join("linux-live-wallpaper")
            .join("sources.txt"));
    }

    let home = std::env::var_os("HOME").context("HOME is not set and XDG_CONFIG_HOME is unset")?;
    Ok(PathBuf::from(home)
        .join(".config")
        .join("linux-live-wallpaper")
        .join("sources.txt"))
}

fn load_sources(path: &PathBuf) -> Result<Vec<String>> {
    match fs::read_to_string(path) {
        Ok(contents) => Ok(contents
            .lines()
            .map(str::trim)
            .filter(|line| !line.is_empty())
            .map(str::to_owned)
            .collect()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(Vec::new()),
        Err(error) => Err(error).with_context(|| format!("could not read {}", path.display())),
    }
}

fn save_sources(path: &PathBuf, sources: &[String]) -> Result<()> {
    let parent = path
        .parent()
        .context("source-list path must have a parent directory")?;
    fs::create_dir_all(parent)
        .with_context(|| format!("could not create {}", parent.display()))?;
    let mut contents = sources.join("\n");
    if !contents.is_empty() {
        contents.push('\n');
    }
    fs::write(path, contents).with_context(|| format!("could not write {}", path.display()))
}

fn load_browsing_mode(path: &Path) -> Result<bool> {
    match fs::read_to_string(path) {
        Ok(value) => Ok(value.trim() == "enabled"),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(error) => Err(error).with_context(|| format!("could not read {}", path.display())),
    }
}

fn save_browsing_mode(path: &Path, enabled: bool) -> Result<()> {
    let parent = path.parent().context("browsing-mode path must have a parent directory")?;
    fs::create_dir_all(parent).with_context(|| format!("could not create {}", parent.display()))?;
    fs::write(path, if enabled { "enabled\n" } else { "disabled\n" })
        .with_context(|| format!("could not write {}", path.display()))
}

fn load_active_source_index(path: &Path, sources: &[String]) -> Result<usize> {
    let saved_source = match fs::read_to_string(path) {
        Ok(source) => source,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(0),
        Err(error) => {
            return Err(error).with_context(|| format!("could not read {}", path.display()));
        }
    };
    let saved_source = saved_source.trim();
    Ok(sources
        .iter()
        .position(|source| source == saved_source)
        .unwrap_or(0))
}

fn save_active_source(path: &Path, source: &str) -> Result<()> {
    let parent = path
        .parent()
        .context("active-source path must have a parent directory")?;
    fs::create_dir_all(parent)
        .with_context(|| format!("could not create {}", parent.display()))?;
    fs::write(path, format!("{source}\n"))
        .with_context(|| format!("could not write {}", path.display()))
}

fn load_source_settings(
    path: &PathBuf,
    defaults: &SourceSettings,
) -> Result<HashMap<String, SourceSettings>> {
    let contents = match fs::read_to_string(path) {
        Ok(contents) => contents,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(HashMap::new()),
        Err(error) => {
            return Err(error).with_context(|| format!("could not read {}", path.display()));
        }
    };
    let mut settings = HashMap::new();
    for (line_number, line) in contents.lines().enumerate() {
        let fields: Vec<_> = line.split('\t').collect();
        if fields.len() != 3 && fields.len() != 4 && fields.len() != 7 && fields.len() != 8 {
            bail!("invalid source settings at {}:{}", path.display(), line_number + 1);
        }
        let url = decode_hex(fields[0]).with_context(|| {
            format!("invalid URL encoding at {}:{}", path.display(), line_number + 1)
        })?;
        let zoom = parse_zoom(fields[1]).with_context(|| {
            format!("invalid zoom setting at {}:{}", path.display(), line_number + 1)
        })?;
        let (opacity, reload_field) = if fields.len() == 4 || fields.len() == 7 || fields.len() == 8 {
            (parse_opacity(fields[2])?, fields[3])
        } else {
            (defaults.opacity, fields[2])
        };
        let reload_seconds = reload_field.parse::<u32>().with_context(|| {
            format!("invalid reload interval at {}:{}", path.display(), line_number + 1)
        })?;
        let mut source = defaults.clone();
        source.zoom = zoom;
        source.opacity = opacity;
        source.reload_seconds = reload_seconds;
        if fields.len() == 7 || fields.len() == 8 {
            source.custom_css = decode_optional_hex(fields[4]).with_context(|| {
                format!("invalid CSS setting at {}:{}", path.display(), line_number + 1)
            })?;
            source.custom_js = decode_optional_hex(fields[5]).with_context(|| {
                format!("invalid JavaScript setting at {}:{}", path.display(), line_number + 1)
            })?;
            source.user_agent = decode_optional_hex(fields[6]).with_context(|| {
                format!("invalid user-agent setting at {}:{}", path.display(), line_number + 1)
            })?;
        }
        if fields.len() == 8 {
            source.monitor_connector = decode_optional_hex(fields[7]).with_context(|| {
                format!("invalid monitor assignment at {}:{}", path.display(), line_number + 1)
            })?;
        }
        settings.insert(url, source);
    }
    Ok(settings)
}

fn save_source_settings(path: &PathBuf, settings: &HashMap<String, SourceSettings>) -> Result<()> {
    let parent = path
        .parent()
        .context("settings path must have a parent directory")?;
    fs::create_dir_all(parent)
        .with_context(|| format!("could not create {}", parent.display()))?;
    let mut urls: Vec<_> = settings.keys().collect();
    urls.sort();
    let mut contents = String::new();
    for url in urls {
        let setting = &settings[url];
        contents.push_str(&format!(
            "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\n",
            encode_hex(url.as_bytes()),
            setting.zoom,
            setting.opacity,
            setting.reload_seconds,
            encode_optional_hex(setting.custom_css.as_deref()),
            encode_optional_hex(setting.custom_js.as_deref()),
            encode_optional_hex(setting.user_agent.as_deref()),
            encode_optional_hex(setting.monitor_connector.as_deref()),
        ));
    }
    fs::write(path, contents).with_context(|| format!("could not write {}", path.display()))
}

fn encode_optional_hex(value: Option<&str>) -> String {
    value
        .map(|value| encode_hex(value.as_bytes()))
        .unwrap_or_else(|| "-".to_owned())
}

fn decode_optional_hex(value: &str) -> Result<Option<String>> {
    if value == "-" {
        Ok(None)
    } else {
        decode_hex(value).map(Some)
    }
}

fn encode_hex(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut encoded = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        encoded.push(HEX[(byte >> 4) as usize] as char);
        encoded.push(HEX[(byte & 0x0f) as usize] as char);
    }
    encoded
}

fn decode_hex(encoded: &str) -> Result<String> {
    if encoded.len() % 2 != 0 {
        bail!("hex URL must contain pairs of digits");
    }
    let mut bytes = Vec::with_capacity(encoded.len() / 2);
    for pair in encoded.as_bytes().chunks_exact(2) {
        let digit = |byte: u8| -> Result<u8> {
            match byte {
                b'0'..=b'9' => Ok(byte - b'0'),
                b'a'..=b'f' => Ok(byte - b'a' + 10),
                b'A'..=b'F' => Ok(byte - b'A' + 10),
                _ => bail!("invalid hexadecimal digit"),
            }
        };
        bytes.push((digit(pair[0])? << 4) | digit(pair[1])?);
    }
    String::from_utf8(bytes).context("URL settings key is not valid UTF-8")
}

fn set_browsing_mode_class(view: &webkit::WebView, enabled: bool) {
    let enabled = if enabled { "true" } else { "false" };
    let script = format!(
        "document.documentElement?.classList.toggle('plash-is-browsing-mode', {enabled});"
    );
    view.evaluate_javascript(
        &script,
        None,
        None,
        None::<&gtk::gio::Cancellable>,
        |_| {},
    );
}

fn apply_source_customizations(
    content_manager: &webkit::UserContentManager,
    base_css: &str,
    source: &SourceSettings,
    web_settings: &webkit::Settings,
) {
    content_manager.remove_all_style_sheets();
    let mut css = base_css.to_owned();
    if let Some(source_css) = source.custom_css.as_deref() {
        css.push_str(source_css);
        css.push('\n');
    }
    let stylesheet = webkit::UserStyleSheet::new(
        &css,
        webkit::UserContentInjectedFrames::TopFrame,
        webkit::UserStyleLevel::User,
        &[],
        &[],
    );
    content_manager.add_style_sheet(&stylesheet);

    content_manager.remove_all_scripts();
    let plash_marker = webkit::UserScript::new(
        "document.documentElement?.classList.add('is-plash-app');",
        webkit::UserContentInjectedFrames::TopFrame,
        webkit::UserScriptInjectionTime::Start,
        &[],
        &[],
    );
    content_manager.add_script(&plash_marker);
    if let Some(source_js) = source.custom_js.as_deref().filter(|script| !script.is_empty()) {
        let script = webkit::UserScript::new(
            source_js,
            webkit::UserContentInjectedFrames::TopFrame,
            webkit::UserScriptInjectionTime::Start,
            &[],
            &[],
        );
        content_manager.add_script(&script);
    }
    web_settings.set_user_agent(source.user_agent.as_deref());
}

fn read_optional_file(path: Option<&PathBuf>) -> Result<Option<String>> {
    path.map(|path| {
        fs::read_to_string(path).with_context(|| format!("could not read {}", path.display()))
    })
    .transpose()
}

fn parse_opacity(value: &str) -> Result<f64> {
    let opacity: f64 = value.parse().context("opacity must be a number from 0.0 to 1.0")?;
    if !opacity.is_finite() || !(0.0..=1.0).contains(&opacity) {
        bail!("opacity must be between 0.0 and 1.0");
    }
    Ok(opacity)
}

fn parse_zoom(value: &str) -> Result<f64> {
    let zoom: f64 = value.parse().context("zoom must be a number from 0.25 to 5.0")?;
    if !zoom.is_finite() || !(0.25..=5.0).contains(&zoom) {
        bail!("zoom must be between 0.25 and 5.0");
    }
    Ok(zoom)
}

fn normalize_tls_host(host: &str) -> Option<String> {
    let host = host.trim();
    if host.is_empty()
        || host.contains("://")
        || host.contains('/')
        || host.contains('@')
        || host.contains(':')
        || host.chars().any(char::is_whitespace)
    {
        return None;
    }
    Some(host.trim_end_matches('.').to_ascii_lowercase())
}

fn is_supported_url(address: &str) -> bool {
    let Some((scheme, remainder)) = address.split_once("://") else {
        return false;
    };
    !remainder.is_empty()
        && !address.chars().any(char::is_whitespace)
        && matches!(scheme.to_ascii_lowercase().as_str(), "http" | "https" | "file")
}

fn build_wallpaper(
    app: &Application,
    urls: &[String],
    config_path: PathBuf,
    monitor_index: Option<u32>,
    rotate_seconds: u32,
    reload_seconds: u32,
    opacity: f64,
    zoom: f64,
    user_agent: Option<&str>,
    allow_self_signed_hosts: &[String],
    invert_colors: bool,
    transparent_background: bool,
    deactivate_on_battery: bool,
    background: bool,
    restore_saved_source: bool,
    custom_css: &Option<String>,
    custom_js: &Option<String>,
) -> Result<()> {
    if urls.is_empty() {
        bail!("at least one wallpaper URL is required");
    }

    let display = gtk::gdk::Display::default().context("no GDK display is available")?;
    let launch_monitor_index = monitor_index.unwrap_or(0);
    let launch_monitor = monitor_at(&display, launch_monitor_index)
        .with_context(|| format!("display index {launch_monitor_index} does not exist"))?;

    let settings_path = config_path.with_file_name("settings.tsv");
    let active_source_path = Rc::new(
        restore_saved_source.then(|| config_path.with_file_name("active-source.txt")),
    );
    let browsing_mode_path = Rc::new(
        restore_saved_source.then(|| config_path.with_file_name("browsing-mode.txt")),
    );
    let initial_browsing_mode = match browsing_mode_path.as_ref().as_deref() {
        Some(path) => load_browsing_mode(path)?,
        None => false,
    };
    let initial_index = match active_source_path.as_ref().as_deref() {
        Some(path) => load_active_source_index(path, urls)?,
        None => 0,
    };
    let default_page_settings = SourceSettings {
        zoom,
        opacity,
        reload_seconds,
        custom_css: custom_css.clone(),
        custom_js: custom_js.clone(),
        user_agent: user_agent.map(str::to_owned),
        monitor_connector: None,
    };
    let mut loaded_settings = load_source_settings(&settings_path, &default_page_settings)?;
    for address in urls {
        loaded_settings
            .entry(address.clone())
            .or_insert_with(|| default_page_settings.clone());
    }
    let source_settings = Rc::new(RefCell::new(loaded_settings));
    let initial_settings = source_settings.borrow()[&urls[initial_index]].clone();
    let monitor = initial_settings
        .monitor_connector
        .as_deref()
        .and_then(|connector| find_monitor_by_connector(&display, connector))
        .unwrap_or_else(|| launch_monitor.clone());
    let selected_monitor = Rc::new(RefCell::new(Some(monitor.clone())));
    let screen_dimensions = Rc::new(Cell::new(monitor_dimensions(&monitor)));

    let content_manager = webkit::UserContentManager::new();
    let mut base_css = String::from(
        "html:has(body > img:only-child), html:has(body > img:only-child) body { width: 100%; height: 100%; margin: 0; padding: 0; overflow: hidden; }\nhtml:has(body > img:only-child) body > img:only-child { display: block !important; width: 100vw !important; height: 100vh !important; object-fit: cover !important; }\n",
    );
    if invert_colors {
        base_css.push_str(
            "\nhtml { filter: invert(1) hue-rotate(180deg) !important; }\nimg, video, canvas, picture, svg image { filter: invert(1) hue-rotate(180deg) !important; }\n",
        );
    }
    let base_css = Rc::new(base_css);
    let web_settings = webkit::Settings::new();
    apply_source_customizations(&content_manager, &base_css, &initial_settings, &web_settings);
    let network_session = webkit::NetworkSession::default()
        .context("could not create a WebKit network session")?;
    network_session.set_tls_errors_policy(webkit::TLSErrorsPolicy::Fail);
    let webview = webkit::WebView::builder()
        .network_session(&network_session)
        .settings(&web_settings)
        .user_content_manager(&content_manager)
        .build();
    if !allow_self_signed_hosts.is_empty() {
        let allowed_hosts = allow_self_signed_hosts.to_vec();
        let network_session = network_session.clone();
        webview.connect_load_failed_with_tls_errors(move |_, failing_uri, certificate, errors| {
            if errors != gtk::gio::TlsCertificateFlags::UNKNOWN_CA {
                return false;
            }
            let Ok(uri) = gtk::glib::Uri::parse(failing_uri, gtk::glib::UriFlags::NONE) else {
                return false;
            };
            let Some(host) = uri.host() else {
                return false;
            };
            let Some(host) = normalize_tls_host(host.as_str()) else {
                return false;
            };
            if allowed_hosts.iter().any(|allowed| allowed == &host) {
                network_session.allow_tls_certificate_for_host(certificate, &host);
                true
            } else {
                false
            }
        });
    }
    webview.set_opacity(source_settings.borrow()[&urls[initial_index]].opacity);
    webview.set_zoom_level(source_settings.borrow()[&urls[initial_index]].zoom);
    if transparent_background {
        webview.set_background_color(&gtk::gdk::RGBA::new(0.0, 0.0, 0.0, 0.0));
    }
    webview.set_is_muted(true);

    let sources = Rc::new(RefCell::new(urls.to_vec()));
    let current = Rc::new(Cell::new(initial_index));
    let browsing = Rc::new(Cell::new(initial_browsing_mode));

    let window = ApplicationWindow::builder()
        .application(app)
        .title("Linux Live Wallpaper")
        .child(&webview)
        .build();

    if transparent_background {
        let provider = gtk::CssProvider::new();
        provider.load_from_data(
            "window.linux-live-wallpaper-transparent { background-color: transparent; }",
        );
        let display = gtk::gdk::Display::default().context("no GDK display is available")?;
        gtk::style_context_add_provider_for_display(
            &display,
            &provider,
            gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );
        window.add_css_class("linux-live-wallpaper-transparent");
    }

    window.init_layer_shell();
    window.set_monitor(&monitor);
    window.set_namespace("linux-live-wallpaper");
    window.set_layer(Layer::Background);
    window.set_anchor(Edge::Top, true);
    window.set_anchor(Edge::Bottom, true);
    window.set_anchor(Edge::Left, true);
    window.set_anchor(Edge::Right, true);
    window.set_keyboard_mode(KeyboardMode::None);
    window.set_exclusive_zone(-1);
    window.present();
    set_wallpaper_interactive(&window, false);

    let status = gtk::Label::new(Some(&sources.borrow()[initial_index]));
    status.set_wrap(true);
    status.set_xalign(0.0);

    let controls = gtk::Box::new(gtk::Orientation::Vertical, 8);
    controls.set_margin_top(12);
    controls.set_margin_bottom(12);
    controls.set_margin_start(12);
    controls.set_margin_end(12);
    controls.append(&status);

    let navigation = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    let previous_button = gtk::Button::with_label("Previous");
    let next_button = gtk::Button::with_label("Next");
    let reload_button = gtk::Button::with_label("Reload");
    navigation.append(&previous_button);
    navigation.append(&next_button);
    navigation.append(&reload_button);
    controls.append(&navigation);

    let history = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    let back_button = gtk::Button::with_label("Back");
    let forward_button = gtk::Button::with_label("Forward");
    back_button.set_sensitive(false);
    forward_button.set_sensitive(false);
    history.append(&back_button);
    history.append(&forward_button);
    controls.append(&history);

    let initial_page_settings =
        source_settings.borrow()[&sources.borrow()[initial_index]].clone();
    let page_settings = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    let zoom_entry = gtk::Entry::new();
    zoom_entry.set_placeholder_text(Some("Zoom (0.25–5.0)"));
    zoom_entry.set_text(&source_settings.borrow()[&sources.borrow()[initial_index]].zoom.to_string());
    let opacity_entry = gtk::Entry::new();
    opacity_entry.set_placeholder_text(Some("Opacity (0.0–1.0)"));
    opacity_entry.set_text(&source_settings.borrow()[&sources.borrow()[initial_index]].opacity.to_string());
    let reload_entry = gtk::Entry::new();
    reload_entry.set_placeholder_text(Some("Reload seconds (0 disables)"));
    reload_entry.set_text(&source_settings.borrow()[&sources.borrow()[initial_index]].reload_seconds.to_string());
    let save_settings_button = gtk::Button::with_label("Save page settings");
    page_settings.append(&zoom_entry);
    page_settings.append(&opacity_entry);
    page_settings.append(&reload_entry);
    controls.append(&page_settings);
    controls.append(&save_settings_button);

    let css_editor = gtk::TextView::new();
    css_editor.set_monospace(true);
    css_editor.set_wrap_mode(gtk::WrapMode::None);
    let css_buffer = css_editor.buffer();
    css_buffer.set_text(initial_page_settings.custom_css.as_deref().unwrap_or(""));
    let css_scroll = gtk::ScrolledWindow::new();
    css_scroll.set_min_content_height(100);
    css_scroll.set_child(Some(&css_editor));
    let css_expander = gtk::Expander::new(Some("Custom CSS for this page"));
    css_expander.set_child(Some(&css_scroll));
    controls.append(&css_expander);

    let js_editor = gtk::TextView::new();
    js_editor.set_monospace(true);
    js_editor.set_wrap_mode(gtk::WrapMode::None);
    let js_buffer = js_editor.buffer();
    js_buffer.set_text(initial_page_settings.custom_js.as_deref().unwrap_or(""));
    let js_scroll = gtk::ScrolledWindow::new();
    js_scroll.set_min_content_height(100);
    js_scroll.set_child(Some(&js_editor));
    let js_expander = gtk::Expander::new(Some("Custom JavaScript for this page"));
    js_expander.set_child(Some(&js_scroll));
    controls.append(&js_expander);

    let user_agent_entry = gtk::Entry::new();
    user_agent_entry.set_placeholder_text(Some("User-Agent (blank uses WebKit default)"));
    user_agent_entry.set_text(initial_page_settings.user_agent.as_deref().unwrap_or(""));
    controls.append(&user_agent_entry);

    let monitor_entry = gtk::Entry::new();
    monitor_entry.set_placeholder_text(Some("Display connector (blank uses launch monitor)"));
    monitor_entry.set_text(initial_page_settings.monitor_connector.as_deref().unwrap_or(""));
    controls.append(&monitor_entry);

    let source_entry = gtk::Entry::new();
    source_entry.set_placeholder_text(Some("https://example.com"));
    controls.append(&source_entry);
    let source_actions = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    let add_button = gtk::Button::with_label("Add source");
    let remove_button = gtk::Button::with_label("Remove active");
    let move_up_button = gtk::Button::with_label("Move up");
    let move_down_button = gtk::Button::with_label("Move down");
    source_actions.append(&add_button);
    source_actions.append(&remove_button);
    controls.append(&source_actions);
    let order_actions = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    order_actions.append(&move_up_button);
    order_actions.append(&move_down_button);
    controls.append(&order_actions);

    let browse_button = gtk::Button::with_label("Enable browsing mode");
    controls.append(&browse_button);
    let quit_button = gtk::Button::with_label("Quit wallpaper");
    controls.append(&quit_button);

    let controls_window = ApplicationWindow::builder()
        .application(app)
        .title("Linux Live Wallpaper")
        .default_width(360)
        .child(&controls)
        .build();

    {
        let view = webview.clone();
        let sources = sources.clone();
        let current = current.clone();
        let status = status.clone();
        let settings = source_settings.clone();
        let zoom_entry = zoom_entry.clone();
        let reload_entry = reload_entry.clone();
        let opacity_entry = opacity_entry.clone();
        let active_source_path = active_source_path.clone();
        let content_manager = content_manager.clone();
        let base_css = base_css.clone();
        let web_settings = web_settings.clone();
        let css_buffer = css_buffer.clone();
        let js_buffer = js_buffer.clone();
        let user_agent_entry = user_agent_entry.clone();
        let monitor_entry = monitor_entry.clone();
        let display = display.clone();
        let window = window.clone();
        let launch_monitor = launch_monitor.clone();
        let selected_monitor = selected_monitor.clone();
        let screen_dimensions = screen_dimensions.clone();
        previous_button.connect_clicked(move |_| {
            let list = sources.borrow();
            let index = (current.get() + list.len() - 1) % list.len();
            drop(list);
            current.set(index);
            load_current(&view, &sources, &current, &status, &settings, &content_manager, &base_css, &web_settings, &css_buffer, &js_buffer, &user_agent_entry, &monitor_entry, &zoom_entry, &opacity_entry, &reload_entry, &display, &window, &launch_monitor, &selected_monitor, &screen_dimensions, active_source_path.as_ref().as_deref());
        });
    }
    {
        let view = webview.clone();
        let sources = sources.clone();
        let current = current.clone();
        let status = status.clone();
        let settings = source_settings.clone();
        let zoom_entry = zoom_entry.clone();
        let reload_entry = reload_entry.clone();
        let opacity_entry = opacity_entry.clone();
        let active_source_path = active_source_path.clone();
        let content_manager = content_manager.clone();
        let base_css = base_css.clone();
        let web_settings = web_settings.clone();
        let css_buffer = css_buffer.clone();
        let js_buffer = js_buffer.clone();
        let user_agent_entry = user_agent_entry.clone();
        let monitor_entry = monitor_entry.clone();
        let display = display.clone();
        let window = window.clone();
        let launch_monitor = launch_monitor.clone();
        let selected_monitor = selected_monitor.clone();
        let screen_dimensions = screen_dimensions.clone();
        next_button.connect_clicked(move |_| {
            let list = sources.borrow();
            let index = (current.get() + 1) % list.len();
            drop(list);
            current.set(index);
            load_current(&view, &sources, &current, &status, &settings, &content_manager, &base_css, &web_settings, &css_buffer, &js_buffer, &user_agent_entry, &monitor_entry, &zoom_entry, &opacity_entry, &reload_entry, &display, &window, &launch_monitor, &selected_monitor, &screen_dimensions, active_source_path.as_ref().as_deref());
        });
    }
    {
        let settings = source_settings.clone();
        let settings_path = settings_path.clone();
        let sources = sources.clone();
        let current = current.clone();
        let view = webview.clone();
        let status = status.clone();
        let zoom_entry = zoom_entry.clone();
        let reload_entry = reload_entry.clone();
        let opacity_entry = opacity_entry.clone();
        let content_manager = content_manager.clone();
        let base_css = base_css.clone();
        let web_settings = web_settings.clone();
        let css_buffer = css_buffer.clone();
        let js_buffer = js_buffer.clone();
        let user_agent_entry = user_agent_entry.clone();
        let monitor_entry = monitor_entry.clone();
        let display = display.clone();
        let window = window.clone();
        let launch_monitor = launch_monitor.clone();
        let selected_monitor = selected_monitor.clone();
        let screen_dimensions = screen_dimensions.clone();
        let active_source_path = active_source_path.clone();
        save_settings_button.connect_clicked(move |_| {
            let parsed_zoom = match parse_zoom(zoom_entry.text().trim()) {
                Ok(value) => value,
                Err(error) => {
                    status.set_text(&format!("{error:#}"));
                    return;
                }
            };
            let parsed_opacity = match parse_opacity(opacity_entry.text().trim()) {
                Ok(value) => value,
                Err(error) => {
                    status.set_text(&format!("{error:#}"));
                    return;
                }
            };
            let parsed_reload = match reload_entry.text().trim().parse::<u32>() {
                Ok(value) => value,
                Err(_) => {
                    status.set_text("Reload interval must be a whole number of seconds.");
                    return;
                }
            };
            let list = sources.borrow();
            let address = list[current.get().min(list.len() - 1)].clone();
            drop(list);
            let css_start = css_buffer.start_iter();
            let css_end = css_buffer.end_iter();
            let css_text = css_buffer.text(&css_start, &css_end, true).to_string();
            let js_start = js_buffer.start_iter();
            let js_end = js_buffer.end_iter();
            let js_text = js_buffer.text(&js_start, &js_end, true).to_string();
            let user_agent_text = user_agent_entry.text().to_string();
            let user_agent = if user_agent_text.trim().is_empty() {
                None
            } else {
                Some(user_agent_text)
            };
            let monitor_text = monitor_entry.text().trim().to_owned();
            let monitor_connector = if monitor_text.is_empty() {
                None
            } else if find_monitor_by_connector(&display, &monitor_text).is_some() {
                Some(monitor_text)
            } else {
                status.set_text("That display connector is not currently connected.");
                return;
            };
            {
                let mut all_settings = settings.borrow_mut();
                let Some(page) = all_settings.get_mut(&address) else {
                    status.set_text("Could not find settings for the active page.");
                    return;
                };
                page.zoom = parsed_zoom;
                page.opacity = parsed_opacity;
                page.reload_seconds = parsed_reload;
                page.custom_css = Some(css_text);
                page.custom_js = Some(js_text);
                page.user_agent = user_agent;
                page.monitor_connector = monitor_connector;
            }
            if let Err(error) = save_source_settings(&settings_path, &settings.borrow()) {
                status.set_text(&format!("Could not save page settings: {error}"));
                return;
            }
            load_current(
                &view,
                &sources,
                &current,
                &status,
                &settings,
                &content_manager,
                &base_css,
                &web_settings,
                &css_buffer,
                &js_buffer,
                &user_agent_entry,
                &monitor_entry,
                &zoom_entry,
                &opacity_entry,
                &reload_entry,
                &display,
                &window,
                &launch_monitor,
                &selected_monitor,
                &screen_dimensions,
                active_source_path.as_ref().as_deref(),
            );
            status.set_text("Saved settings for this page.");
        });
    }

    {
        let view = webview.clone();
        reload_button.connect_clicked(move |_| view.reload());
    }
    {
        let view = webview.clone();
        back_button.connect_clicked(move |_| {
            if view.can_go_back() {
                view.go_back();
            }
        });
    }
    {
        let view = webview.clone();
        forward_button.connect_clicked(move |_| {
            if view.can_go_forward() {
                view.go_forward();
            }
        });
    }
    {
        let back_button = back_button.clone();
        let forward_button = forward_button.clone();
        let status = status.clone();
        let browsing = browsing.clone();
        webview.connect_load_changed(move |view, event| {
            back_button.set_sensitive(view.can_go_back());
            forward_button.set_sensitive(view.can_go_forward());
            match event {
                webkit::LoadEvent::Started => {
                    let uri = view.uri().map(|uri| uri.to_string()).unwrap_or_default();
                    status.set_text(&format!("Loading {uri}"));
                }
                webkit::LoadEvent::Committed => {
                    let uri = view.uri().map(|uri| uri.to_string()).unwrap_or_default();
                    status.set_text(&format!("Loaded response from {uri}"));
                }
                webkit::LoadEvent::Finished => {
                    set_browsing_mode_class(view, browsing.get());
                    let uri = view.uri().map(|uri| uri.to_string()).unwrap_or_default();
                    status.set_text(&uri);
                }
                _ => {}
            }
        });
    }
    {
        let status = status.clone();
        webview.connect_load_failed(move |_, _event, failing_uri, error| {
            let message = format!("Could not load {failing_uri}: {error}");
            eprintln!("{message}");
            status.set_text(&message);
            false
        });
    }
    let initial_url = resolve_url(&urls[initial_index], screen_dimensions.get());
    webview.load_uri(&initial_url);

    {
        let path = config_path.clone();
        let sources = sources.clone();
        let current = current.clone();
        let entry = source_entry.clone();
        let view = webview.clone();
        let status = status.clone();
        let settings = source_settings.clone();
        let settings_path = settings_path.clone();
        let zoom_entry = zoom_entry.clone();
        let reload_entry = reload_entry.clone();
        let opacity_entry = opacity_entry.clone();
        let active_source_path = active_source_path.clone();
        let content_manager = content_manager.clone();
        let base_css = base_css.clone();
        let web_settings = web_settings.clone();
        let css_buffer = css_buffer.clone();
        let js_buffer = js_buffer.clone();
        let user_agent_entry = user_agent_entry.clone();
        let default_page_settings = default_page_settings.clone();
        let monitor_entry = monitor_entry.clone();
        let display = display.clone();
        let window = window.clone();
        let launch_monitor = launch_monitor.clone();
        let selected_monitor = selected_monitor.clone();
        let screen_dimensions = screen_dimensions.clone();
        add_button.connect_clicked(move |_| {
            let address = entry.text().trim().to_owned();
            if !is_supported_url(&address) {
                status.set_text("Enter a valid http://, https:// or file:// URL.");
                return;
            }
            let mut updated = sources.borrow().clone();
            if updated.contains(&address) {
                status.set_text("That source is already in the list.");
                return;
            }
            updated.push(address.clone());
            if let Err(error) = save_sources(&path, &updated) {
                status.set_text(&format!("Could not save sources: {error}"));
                return;
            }
            *sources.borrow_mut() = updated;
            settings
                .borrow_mut()
                .insert(
                    address,
                    default_page_settings.clone(),
                );
            if let Err(error) = save_source_settings(&settings_path, &settings.borrow()) {
                status.set_text(&format!("Could not save page settings: {error}"));
                return;
            }
            current.set(sources.borrow().len() - 1);
            load_current(&view, &sources, &current, &status, &settings, &content_manager, &base_css, &web_settings, &css_buffer, &js_buffer, &user_agent_entry, &monitor_entry, &zoom_entry, &opacity_entry, &reload_entry, &display, &window, &launch_monitor, &selected_monitor, &screen_dimensions, active_source_path.as_ref().as_deref());
            entry.set_text("");
        });
    }
    {
        let path = config_path.clone();
        let sources = sources.clone();
        let current = current.clone();
        let view = webview.clone();
        let status = status.clone();
        let settings = source_settings.clone();
        let settings_path = settings_path.clone();
        let zoom_entry = zoom_entry.clone();
        let reload_entry = reload_entry.clone();
        let opacity_entry = opacity_entry.clone();
        let active_source_path = active_source_path.clone();
        let content_manager = content_manager.clone();
        let base_css = base_css.clone();
        let web_settings = web_settings.clone();
        let css_buffer = css_buffer.clone();
        let js_buffer = js_buffer.clone();
        let user_agent_entry = user_agent_entry.clone();
        let monitor_entry = monitor_entry.clone();
        let display = display.clone();
        let window = window.clone();
        let launch_monitor = launch_monitor.clone();
        let selected_monitor = selected_monitor.clone();
        let screen_dimensions = screen_dimensions.clone();
        remove_button.connect_clicked(move |_| {
            let mut updated = sources.borrow().clone();
            if updated.len() <= 1 {
                status.set_text("Keep at least one source in the list.");
                return;
            }
            let removed_url = updated.remove(current.get().min(updated.len() - 1));
            if let Err(error) = save_sources(&path, &updated) {
                status.set_text(&format!("Could not save sources: {error}"));
                return;
            }
            *sources.borrow_mut() = updated;
            settings.borrow_mut().remove(&removed_url);
            if let Err(error) = save_source_settings(&settings_path, &settings.borrow()) {
                status.set_text(&format!("Could not save page settings: {error}"));
                return;
            }
            current.set(current.get().min(sources.borrow().len() - 1));
            load_current(&view, &sources, &current, &status, &settings, &content_manager, &base_css, &web_settings, &css_buffer, &js_buffer, &user_agent_entry, &monitor_entry, &zoom_entry, &opacity_entry, &reload_entry, &display, &window, &launch_monitor, &selected_monitor, &screen_dimensions, active_source_path.as_ref().as_deref());
        });
    }
    for (button, direction) in [(&move_up_button, -1isize), (&move_down_button, 1isize)] {
        let path = config_path.clone();
        let sources = sources.clone();
        let current = current.clone();
        let view = webview.clone();
        let status = status.clone();
        let settings = source_settings.clone();
        let zoom_entry = zoom_entry.clone();
        let reload_entry = reload_entry.clone();
        let opacity_entry = opacity_entry.clone();
        let active_source_path = active_source_path.clone();
        let content_manager = content_manager.clone();
        let base_css = base_css.clone();
        let web_settings = web_settings.clone();
        let css_buffer = css_buffer.clone();
        let js_buffer = js_buffer.clone();
        let user_agent_entry = user_agent_entry.clone();
        let monitor_entry = monitor_entry.clone();
        let display = display.clone();
        let window = window.clone();
        let launch_monitor = launch_monitor.clone();
        let selected_monitor = selected_monitor.clone();
        let screen_dimensions = screen_dimensions.clone();
        button.connect_clicked(move |_| {
            let mut updated = sources.borrow().clone();
            let index = current.get();
            let target = index as isize + direction;
            if target < 0 || target >= updated.len() as isize {
                return;
            }
            updated.swap(index, target as usize);
            if let Err(error) = save_sources(&path, &updated) {
                status.set_text(&format!("Could not save sources: {error}"));
                return;
            }
            *sources.borrow_mut() = updated;
            current.set(target as usize);
            load_current(&view, &sources, &current, &status, &settings, &content_manager, &base_css, &web_settings, &css_buffer, &js_buffer, &user_agent_entry, &monitor_entry, &zoom_entry, &opacity_entry, &reload_entry, &display, &window, &launch_monitor, &selected_monitor, &screen_dimensions, active_source_path.as_ref().as_deref());
        });
    }

    {
        let monitors = display.monitors();
        let sources = sources.clone();
        let current = current.clone();
        let settings = source_settings.clone();
        let selected_monitor = selected_monitor.clone();
        let display = display.clone();
        let window = window.clone();
        let controls_window = controls_window.clone();
        let status = status.clone();
        let view = webview.clone();
        let content_manager = content_manager.clone();
        let base_css = base_css.clone();
        let web_settings = web_settings.clone();
        let css_buffer = css_buffer.clone();
        let js_buffer = js_buffer.clone();
        let user_agent_entry = user_agent_entry.clone();
        let monitor_entry = monitor_entry.clone();
        let zoom_entry = zoom_entry.clone();
        let opacity_entry = opacity_entry.clone();
        let reload_entry = reload_entry.clone();
        let launch_monitor = launch_monitor.clone();
        let screen_dimensions = screen_dimensions.clone();
        let active_source_path = active_source_path.clone();
        monitors.connect_items_changed(move |_, _, _, _| {
            if monitor_at(&display, 0).is_none() {
                *selected_monitor.borrow_mut() = None;
                window.set_visible(false);
                controls_window.set_visible(false);
                status.set_text("No displays connected; waiting for a display.");
                return;
            }
            let visible = !deactivate_on_battery || !is_on_battery();
            window.set_visible(visible);
            if !background {
                controls_window.set_visible(visible);
            }
            load_current(
                &view, &sources, &current, &status, &settings, &content_manager,
                &base_css, &web_settings, &css_buffer, &js_buffer, &user_agent_entry,
                &monitor_entry, &zoom_entry, &opacity_entry, &reload_entry, &display,
                &window, &launch_monitor, &selected_monitor, &screen_dimensions,
                active_source_path.as_ref().as_deref(),
            );
        });
    }

    set_wallpaper_interactive(&window, initial_browsing_mode);
    browse_button.set_label(if initial_browsing_mode {
        "Disable browsing mode"
    } else {
        "Enable browsing mode"
    });
    {
        let wallpaper = window.clone();
        let view = webview.clone();
        let browsing = browsing.clone();
        let browsing_mode_path = browsing_mode_path.clone();
        let status = status.clone();
        browse_button.connect_clicked(move |button| {
            let enabled = !browsing.get();
            browsing.set(enabled);
            set_browsing_mode_class(&view, enabled);
            if let Some(path) = browsing_mode_path.as_ref().as_deref() {
                if let Err(error) = save_browsing_mode(path, enabled) {
                    status.set_text(&format!("Changed browsing mode, but could not save it: {error:#}"));
                }
            }
            set_wallpaper_interactive(&wallpaper, enabled);
            button.set_label(if enabled {
                "Disable browsing mode"
            } else {
                "Enable browsing mode"
            });
        });
    }
    {
        let app = app.clone();
        quit_button.connect_clicked(move |_| app.quit());
    }
    {
        let app = app.clone();
        let wallpaper = window.clone();
        let browsing = browsing.clone();
        controls_window.connect_close_request(move |_| {
            set_wallpaper_interactive(&wallpaper, false);
            browsing.set(false);
            app.quit();
            gtk::glib::Propagation::Proceed
        });
    }

    let key_controller = gtk::EventControllerKey::new();
    {
        let view = webview.clone();
        let sources = sources.clone();
        let current = current.clone();
        let status = status.clone();
        let settings = source_settings.clone();
        let zoom_entry = zoom_entry.clone();
        let reload_entry = reload_entry.clone();
        let opacity_entry = opacity_entry.clone();
        let wallpaper = window.clone();
        let browsing = browsing.clone();
        let browse_button = browse_button.clone();
        let browsing_mode_path = browsing_mode_path.clone();
        let active_source_path = active_source_path.clone();
        let content_manager = content_manager.clone();
        let base_css = base_css.clone();
        let web_settings = web_settings.clone();
        let css_buffer = css_buffer.clone();
        let js_buffer = js_buffer.clone();
        let user_agent_entry = user_agent_entry.clone();
        let monitor_entry = monitor_entry.clone();
        let display = display.clone();
        let window = window.clone();
        let launch_monitor = launch_monitor.clone();
        let selected_monitor = selected_monitor.clone();
        let screen_dimensions = screen_dimensions.clone();
        key_controller.connect_key_pressed(move |_, key, _, modifiers| {
            use gtk::gdk::{Key, ModifierType};

            if modifiers.contains(ModifierType::CONTROL_MASK) && key == Key::r {
                view.reload();
                gtk::glib::Propagation::Stop
            } else if modifiers.contains(ModifierType::ALT_MASK) && key == Key::Left {
                let list = sources.borrow();
                let index = (current.get() + list.len() - 1) % list.len();
                drop(list);
                current.set(index);
                load_current(&view, &sources, &current, &status, &settings, &content_manager, &base_css, &web_settings, &css_buffer, &js_buffer, &user_agent_entry, &monitor_entry, &zoom_entry, &opacity_entry, &reload_entry, &display, &window, &launch_monitor, &selected_monitor, &screen_dimensions, active_source_path.as_ref().as_deref());
                gtk::glib::Propagation::Stop
            } else if modifiers.contains(ModifierType::ALT_MASK) && key == Key::Right {
                let list = sources.borrow();
                let index = (current.get() + 1) % list.len();
                drop(list);
                current.set(index);
                load_current(&view, &sources, &current, &status, &settings, &content_manager, &base_css, &web_settings, &css_buffer, &js_buffer, &user_agent_entry, &monitor_entry, &zoom_entry, &opacity_entry, &reload_entry, &display, &window, &launch_monitor, &selected_monitor, &screen_dimensions, active_source_path.as_ref().as_deref());
                gtk::glib::Propagation::Stop
            } else if modifiers.contains(ModifierType::CONTROL_MASK) && key == Key::b {
                let enabled = !browsing.get();
                browsing.set(enabled);
                set_browsing_mode_class(&view, enabled);
                if let Some(path) = browsing_mode_path.as_ref().as_deref() {
                    if let Err(error) = save_browsing_mode(path, enabled) {
                        status.set_text(&format!("Changed browsing mode, but could not save it: {error:#}"));
                    }
                }
                set_wallpaper_interactive(&wallpaper, enabled);
                browse_button.set_label(if enabled {
                    "Disable browsing mode"
                } else {
                    "Enable browsing mode"
                });
                gtk::glib::Propagation::Stop
            } else {
                gtk::glib::Propagation::Proceed
            }
        });
    }
    controls_window.add_controller(key_controller);

    if rotate_seconds > 0 {
        let sources = sources.clone();
        let current = current.clone();
        let view = webview.clone();
        let status = status.clone();
        let settings = source_settings.clone();
        let zoom_entry = zoom_entry.clone();
        let reload_entry = reload_entry.clone();
        let opacity_entry = opacity_entry.clone();
        let active_source_path = active_source_path.clone();
        let content_manager = content_manager.clone();
        let base_css = base_css.clone();
        let web_settings = web_settings.clone();
        let css_buffer = css_buffer.clone();
        let js_buffer = js_buffer.clone();
        let user_agent_entry = user_agent_entry.clone();
        let monitor_entry = monitor_entry.clone();
        let display = display.clone();
        let window = window.clone();
        let launch_monitor = launch_monitor.clone();
        let selected_monitor = selected_monitor.clone();
        let screen_dimensions = screen_dimensions.clone();
        gtk::glib::timeout_add_seconds_local(rotate_seconds, move || {
            let list = sources.borrow();
            if list.len() > 1 {
                let index = (current.get() + 1) % list.len();
                drop(list);
                current.set(index);
                load_current(&view, &sources, &current, &status, &settings, &content_manager, &base_css, &web_settings, &css_buffer, &js_buffer, &user_agent_entry, &monitor_entry, &zoom_entry, &opacity_entry, &reload_entry, &display, &window, &launch_monitor, &selected_monitor, &screen_dimensions, active_source_path.as_ref().as_deref());
            }
            gtk::glib::ControlFlow::Continue
        });
    }

    {
        let sources = sources.clone();
        let current = current.clone();
        let settings = source_settings.clone();
        let view = webview.clone();
        let wallpaper_window = window.clone();
        let mut elapsed = 0u32;
        let mut last_index = current.get();
        gtk::glib::timeout_add_seconds_local(1, move || {
            let index = current.get();
            if index != last_index {
                last_index = index;
                elapsed = 0;
            }
            let interval = {
                let list = sources.borrow();
                list.get(index)
                    .and_then(|address| {
                        settings
                            .borrow()
                            .get(address)
                            .map(|setting| setting.reload_seconds)
                    })
                    .unwrap_or(0)
            };
            if interval == 0 {
                elapsed = 0;
            } else if wallpaper_window.is_visible() {
                elapsed = elapsed.saturating_add(1);
                if elapsed >= interval {
                    view.reload();
                    elapsed = 0;
                }
            }
            gtk::glib::ControlFlow::Continue
        });
    }

    if !background {
        controls_window.present();
    } else {
        controls_window.set_visible(false);
    }

    if deactivate_on_battery {
        let wallpaper_window = window.clone();
        let battery_controls_window = controls_window.clone();
        gtk::glib::timeout_add_seconds_local(30, move || {
            let visible = !is_on_battery();
            wallpaper_window.set_visible(visible);
            if !background {
                battery_controls_window.set_visible(visible);
            }
            gtk::glib::ControlFlow::Continue
        });
        if is_on_battery() {
            window.set_visible(false);
            controls_window.set_visible(false);
        }
    }
    Ok(())
}

fn load_current(
    view: &webkit::WebView,
    sources: &RefCell<Vec<String>>,
    current: &Cell<usize>,
    status: &gtk::Label,
    settings: &RefCell<HashMap<String, SourceSettings>>,
    content_manager: &webkit::UserContentManager,
    base_css: &str,
    web_settings: &webkit::Settings,
    css_buffer: &gtk::TextBuffer,
    js_buffer: &gtk::TextBuffer,
    user_agent_entry: &gtk::Entry,
    monitor_entry: &gtk::Entry,
    zoom_entry: &gtk::Entry,
    opacity_entry: &gtk::Entry,
    reload_entry: &gtk::Entry,
    display: &gtk::gdk::Display,
    window: &ApplicationWindow,
    launch_monitor: &gtk::gdk::Monitor,
    selected_monitor: &RefCell<Option<gtk::gdk::Monitor>>,
    screen_dimensions: &Cell<(u32, u32)>,
    active_source_path: Option<&Path>,
) {
    let list = sources.borrow();
    if list.is_empty() {
        return;
    }
    let index = current.get().min(list.len() - 1);
    current.set(index);
    let address = list[index].clone();
    drop(list);
    let setting = settings.borrow().get(&address).cloned().unwrap_or(SourceSettings {
        zoom: 1.0,
        opacity: 1.0,
        reload_seconds: 0,
        custom_css: None,
        custom_js: None,
        user_agent: None,
        monitor_connector: None,
    });
    let requested_connector = setting.monitor_connector.as_deref();
    let selected = requested_connector
        .and_then(|connector| find_monitor_by_connector(display, connector))
        .or_else(|| {
            launch_monitor
                .connector()
                .as_deref()
                .and_then(|connector| find_monitor_by_connector(display, connector))
        })
        .or_else(|| monitor_at(display, 0));
    let Some(monitor) = selected else {
        status.set_text("No display is available for the wallpaper.");
        return;
    };
    window.set_monitor(&monitor);
    *selected_monitor.borrow_mut() = Some(monitor.clone());
    let dimensions = monitor_dimensions(&monitor);
    screen_dimensions.set(dimensions);
    let resolved_address = resolve_url(&address, dimensions);
    apply_source_customizations(content_manager, base_css, &setting, web_settings);
    view.load_uri(&resolved_address);
    view.set_zoom_level(setting.zoom);
    view.set_opacity(setting.opacity);
    if requested_connector.is_some() && monitor.connector().as_deref() != requested_connector {
        status.set_text(&format!(
            "{address} (display {requested_connector:?} is disconnected; using {})",
            monitor.connector().as_deref().unwrap_or("current output")
        ));
    } else {
        status.set_text(&address);
    }
    if let Some(path) = active_source_path {
        if let Err(error) = save_active_source(path, &address) {
            status.set_text(&format!("Selected page, but could not save resume state: {error:#}"));
        }
    }
    zoom_entry.set_text(&setting.zoom.to_string());
    opacity_entry.set_text(&setting.opacity.to_string());
    reload_entry.set_text(&setting.reload_seconds.to_string());
    css_buffer.set_text(setting.custom_css.as_deref().unwrap_or(""));
    js_buffer.set_text(setting.custom_js.as_deref().unwrap_or(""));
    user_agent_entry.set_text(setting.user_agent.as_deref().unwrap_or(""));
    monitor_entry.set_text(setting.monitor_connector.as_deref().unwrap_or(""));
}

fn resolve_url(address: &str, dimensions: (u32, u32)) -> String {
    address
        .replace("[[screenWidth]]", &dimensions.0.to_string())
        .replace("[[screenHeight]]", &dimensions.1.to_string())
}

fn set_wallpaper_interactive(window: &ApplicationWindow, enabled: bool) {
    if enabled {
        window.set_keyboard_mode(KeyboardMode::OnDemand);
    } else {
        window.set_keyboard_mode(KeyboardMode::None);
    }

    if let Some(surface) = window.surface() {
        let region = if enabled {
            gtk::cairo::Region::create_rectangle(&gtk::cairo::RectangleInt::new(
                0,
                0,
                window.width().max(1),
                window.height().max(1),
            ))
        } else {
            gtk::cairo::Region::create()
        };
        surface.set_input_region(&region);
    }
}

fn is_on_battery() -> bool {
    let Ok(supplies) = fs::read_dir("/sys/class/power_supply") else {
        return false;
    };

    supplies.flatten().any(|entry| {
        let path = entry.path();
        let Ok(kind) = fs::read_to_string(path.join("type")) else {
            return false;
        };
        if !kind.trim().eq_ignore_ascii_case("battery") {
            return false;
        }
        fs::read_to_string(path.join("status"))
            .map(|status| status.trim().eq_ignore_ascii_case("discharging"))
            .unwrap_or(false)
    })
}

#[cfg(test)]
mod tests {
    use super::{is_supported_url, parse_opacity};

    #[test]
    fn accepts_http_https_and_file_urls() {
        assert!(is_supported_url("http://example.com"));
        assert!(is_supported_url("https://example.com"));
        assert!(is_supported_url("file:///home/user/wallpaper.html"));
    }

    #[test]
    fn rejects_missing_or_unsupported_schemes() {
        assert!(!is_supported_url("example.com"));
        assert!(!is_supported_url("javascript://alert(1)"));
        assert!(!is_supported_url("https://"));
        assert!(!is_supported_url("data://text/html,hello"));
        assert!(!is_supported_url("https://example.com/a b"));
    }

    #[test]
    fn validates_opacity_range() {
        assert_eq!(parse_opacity("0.5").unwrap(), 0.5);
        assert_eq!(parse_opacity("0").unwrap(), 0.0);
        assert_eq!(parse_opacity("1").unwrap(), 1.0);
        assert!(parse_opacity("-0.1").is_err());
        assert!(parse_opacity("1.1").is_err());
        assert!(parse_opacity("NaN").is_err());
    }
}
