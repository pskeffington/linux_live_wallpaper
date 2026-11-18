use anyhow::{bail, Context, Result};
use clap::Parser;
use gtk::gio::prelude::*;
use gtk::prelude::*;
use gtk::{Application, ApplicationWindow};
use gtk4_layer_shell::{Edge, KeyboardMode, Layer, LayerShell};
use std::cell::Cell;
use std::fs;
use std::path::PathBuf;
use std::rc::Rc;
use webkit::prelude::*;

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

    /// Custom HTTP User-Agent for loaded web pages.
    #[arg(long, value_name = "STRING")]
    user_agent: Option<String>,

    /// Request a transparent web and GTK window background (compositor support varies).
    #[arg(long)]
    transparent_background: bool,

    /// Hide the wallpaper while a Linux battery reports discharging.
    #[arg(long)]
    deactivate_on_battery: bool,
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

    let urls = if args.urls.is_empty() {
        if saved_sources.is_empty() {
            vec!["https://example.com".to_owned()]
        } else {
            saved_sources
        }
    } else {
        args.urls
    };

    if urls.iter().any(|url| !is_supported_url(url)) {
        bail!("only http://, https:// and file:// wallpaper URLs are supported");
    }

    let opacity = parse_opacity(&args.opacity)?;
    let zoom = parse_zoom(&args.zoom)?;
    let user_agent = args.user_agent;
    let custom_css = read_optional_file(args.css_file.as_ref())?;
    let custom_js = read_optional_file(args.js_file.as_ref())?;
    let monitor = args.monitor;
    let rotate_seconds = args.rotate_seconds;
    let reload_seconds = args.reload_seconds;
    let invert_colors = args.invert_colors;
    let transparent_background = args.transparent_background;
    let deactivate_on_battery = args.deactivate_on_battery;

    let app = Application::builder()
        .application_id("io.github.pskeffington.LinuxLiveWallpaper")
        .build();
    app.connect_activate(move |app| {
        if let Err(error) = build_wallpaper(
            app,
            &urls,
            monitor,
            rotate_seconds,
            reload_seconds,
            opacity,
            zoom,
            user_agent.as_deref(),
            invert_colors,
            transparent_background,
            deactivate_on_battery,
            &custom_css,
            &custom_js,
        ) {
            eprintln!("Could not create wallpaper surface: {error:#}");
        }
    });
    app.run();
    Ok(())
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
    monitor_index: Option<u32>,
    rotate_seconds: u32,
    reload_seconds: u32,
    opacity: f64,
    zoom: f64,
    user_agent: Option<&str>,
    invert_colors: bool,
    transparent_background: bool,
    deactivate_on_battery: bool,
    custom_css: &Option<String>,
    custom_js: &Option<String>,
) -> Result<()> {
    if urls.is_empty() {
        bail!("at least one wallpaper URL is required");
    }

    let content_manager = webkit::UserContentManager::new();
    let mut css = custom_css.clone().unwrap_or_default();
    if invert_colors {
        css.push_str(
            "\\nhtml { filter: invert(1) hue-rotate(180deg) !important; }\\nimg, video, canvas, picture, svg image { filter: invert(1) hue-rotate(180deg) !important; }\\n",
        );
    }
    if !css.is_empty() {
        let stylesheet = webkit::UserStyleSheet::new(
            &css,
            webkit::UserContentInjectedFrames::TopFrame,
            webkit::UserStyleLevel::User,
            &[],
            &[],
        );
        content_manager.add_style_sheet(&stylesheet);
    }
    if let Some(script) = custom_js {
        let script = webkit::UserScript::new(
            script,
            webkit::UserContentInjectedFrames::TopFrame,
            webkit::UserScriptInjectionTime::Start,
            &[],
            &[],
        );
        content_manager.add_script(&script);
    }

    let settings = webkit::Settings::new();
    if let Some(user_agent) = user_agent {
        settings.set_user_agent(Some(user_agent));
    }
    let webview = webkit::WebView::builder()
        .settings(&settings)
        .user_content_manager(&content_manager)
        .build();
    webview.set_opacity(opacity);
    webview.set_zoom_level(zoom);
    if transparent_background {
        webview.set_background_color(&gtk::gdk::RGBA::new(0.0, 0.0, 0.0, 0.0));
    }
    webview.set_is_muted(true);
    webview.load_uri(&urls[0]);

    let sources = Rc::new(urls.to_vec());
    let current = Rc::new(Cell::new(0usize));

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
    if let Some(index) = monitor_index {
        let display = gtk::gdk::Display::default().context("no GDK display is available")?;
        let monitor = display
            .monitors()
            .item(index)
            .and_then(|item| item.downcast::<gtk::gdk::Monitor>().ok())
            .with_context(|| format!("display index {index} does not exist"))?;
        window.set_monitor(&monitor);
    }
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

    let status = gtk::Label::new(Some(&sources[0]));
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
        previous_button.connect_clicked(move |_| {
            let index = (current.get() + sources.len() - 1) % sources.len();
            current.set(index);
            view.load_uri(&sources[index]);
            status.set_text(&sources[index]);
        });
    }
    {
        let view = webview.clone();
        let sources = sources.clone();
        let current = current.clone();
        let status = status.clone();
        next_button.connect_clicked(move |_| {
            let index = (current.get() + 1) % sources.len();
            current.set(index);
            view.load_uri(&sources[index]);
            status.set_text(&sources[index]);
        });
    }
    {
        let view = webview.clone();
        reload_button.connect_clicked(move |_| view.reload());
    }

    let browsing = Rc::new(Cell::new(false));
    {
        let wallpaper = window.clone();
        let browsing = browsing.clone();
        browse_button.connect_clicked(move |button| {
            let enabled = !browsing.get();
            browsing.set(enabled);
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
        let wallpaper = window.clone();
        let browsing = browsing.clone();
        let browse_button = browse_button.clone();
        key_controller.connect_key_pressed(move |_, key, _, modifiers| {
            use gtk::gdk::{Key, ModifierType};

            if modifiers.contains(ModifierType::CONTROL_MASK) && key == Key::r {
                view.reload();
                gtk::glib::Propagation::Stop
            } else if modifiers.contains(ModifierType::ALT_MASK) && key == Key::Left {
                let index = (current.get() + sources.len() - 1) % sources.len();
                current.set(index);
                view.load_uri(&sources[index]);
                status.set_text(&sources[index]);
                gtk::glib::Propagation::Stop
            } else if modifiers.contains(ModifierType::ALT_MASK) && key == Key::Right {
                let index = (current.get() + 1) % sources.len();
                current.set(index);
                view.load_uri(&sources[index]);
                status.set_text(&sources[index]);
                gtk::glib::Propagation::Stop
            } else if modifiers.contains(ModifierType::CONTROL_MASK) && key == Key::b {
                let enabled = !browsing.get();
                browsing.set(enabled);
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

    if rotate_seconds > 0 && sources.len() > 1 {
        let sources = sources.clone();
        let current = current.clone();
        let view = webview.clone();
        let status = status.clone();
        gtk::glib::timeout_add_seconds_local(rotate_seconds, move || {
            let index = (current.get() + 1) % sources.len();
            current.set(index);
            view.load_uri(&sources[index]);
            status.set_text(&sources[index]);
            gtk::glib::ControlFlow::Continue
        });
    }

    if reload_seconds > 0 {
        let view = webview.clone();
        gtk::glib::timeout_add_seconds_local(reload_seconds, move || {
            view.reload();
            gtk::glib::ControlFlow::Continue
        });
    }

    controls_window.present();

    if deactivate_on_battery {
        let wallpaper_window = window.clone();
        let controls_window = controls_window.clone();
        gtk::glib::timeout_add_seconds_local(30, move || {
            let visible = !is_on_battery();
            wallpaper_window.set_visible(visible);
            controls_window.set_visible(visible);
            gtk::glib::ControlFlow::Continue
        });
        if is_on_battery() {
            window.set_visible(false);
            controls_window.set_visible(false);
        }
    }
    Ok(())
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
