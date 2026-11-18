use anyhow::{bail, Context, Result};
use clap::Parser;
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

    /// Seconds between automatic switches. Zero disables rotation.
    #[arg(long, default_value_t = 0, value_parser = clap::value_parser!(u32).range(0..))]
    rotate_seconds: u32,
}

fn main() -> Result<()> {
    let args = Args::parse();
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

    let invalid: Vec<_> = urls.iter().filter(|url| !is_supported_url(url)).collect();
    if !invalid.is_empty() {
        bail!("only http://, https:// and file:// wallpaper URLs are supported");
    }

    let app = Application::builder()
        .application_id("io.github.pskeffington.LinuxLiveWallpaper")
        .build();
    app.connect_activate(move |app| {
        if let Err(error) = build_wallpaper(app, &urls, args.rotate_seconds) {
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

fn is_supported_url(address: &str) -> bool {
    let Some((scheme, remainder)) = address.split_once("://") else {
        return false;
    };
    !remainder.is_empty()
        && !address.chars().any(char::is_whitespace)
        && matches!(scheme.to_ascii_lowercase().as_str(), "http" | "https" | "file")
}

fn build_wallpaper(app: &Application, urls: &[String], rotate_seconds: u32) -> Result<()> {
    if urls.is_empty() {
        bail!("at least one wallpaper URL is required");
    }

    let webview = webkit::WebView::new();
    webview.load_uri(&urls[0]);

    if rotate_seconds > 0 && urls.len() > 1 {
        let sources = Rc::new(urls.to_vec());
        let current = Rc::new(Cell::new(0usize));
        let view = webview.clone();
        gtk::glib::timeout_add_seconds_local(rotate_seconds, move || {
            let next = (current.get() + 1) % sources.len();
            current.set(next);
            view.load_uri(&sources[next]);
            gtk::glib::ControlFlow::Continue
        });
    }

    let window = ApplicationWindow::builder()
        .application(app)
        .title("Linux Live Wallpaper")
        .child(&webview)
        .build();

    window.init_layer_shell();
    window.set_namespace("linux-live-wallpaper");
    window.set_layer(Layer::Background);
    window.set_anchor(Edge::Top, true);
    window.set_anchor(Edge::Bottom, true);
    window.set_anchor(Edge::Left, true);
    window.set_anchor(Edge::Right, true);
    window.set_keyboard_mode(KeyboardMode::None);
    window.set_exclusive_zone(-1);
    window.present();
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::is_supported_url;

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
}
