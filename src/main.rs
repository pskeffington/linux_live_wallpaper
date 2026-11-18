use anyhow::{bail, Result};
use clap::Parser;
use gtk::prelude::*;
use gtk::{Application, ApplicationWindow};
use gtk4_layer_shell::{Edge, KeyboardMode, Layer, LayerShell};
use std::cell::Cell;
use std::rc::Rc;
use webkit::prelude::*;

#[derive(Debug, Parser)]
#[command(name = "linux-live-wallpaper", version, about)]
struct Args {
    /// Website(s) to show as the live wallpaper. Repeat the option to add sources.
    #[arg(long = "url", value_name = "URL", num_args = 1..)]
    urls: Vec<String>,

    /// Seconds between automatic switches. Zero disables rotation.
    #[arg(long, default_value_t = 0, value_parser = clap::value_parser!(u32).range(0..))]
    rotate_seconds: u32,
}

fn main() {
    let args = Args::parse();
    let urls = if args.urls.is_empty() {
        vec!["https://example.com".to_owned()]
    } else {
        args.urls
    };

    let invalid: Vec<_> = urls.iter().filter(|url| !is_supported_url(url)).collect();
    if !invalid.is_empty() {
        eprintln!("Only http://, https:// and file:// wallpaper URLs are supported.");
        std::process::exit(2);
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
}

fn is_supported_url(address: &str) -> bool {
    let Some((scheme, remainder)) = address.split_once("://") else {
        return false;
    };
    !remainder.is_empty()
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
    }
}
