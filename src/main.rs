use anyhow::{Context, Result};
use clap::Parser;
use gtk::prelude::*;
use gtk::{Application, ApplicationWindow};
use gtk4_layer_shell::{Edge, KeyboardMode, Layer, LayerShell};
use webkit::prelude::*;

#[derive(Debug, Parser)]
#[command(name = "linux-live-wallpaper", version, about)]
struct Args {
    /// Website to show as the live wallpaper.
    #[arg(long, default_value = "https://example.com")]
    url: String,
}

fn main() -> Result<()> {
    let args = Args::parse();
    let app = Application::builder()
        .application_id("io.github.pskeffington.LinuxLiveWallpaper")
        .build();
    let url = args.url;

    app.connect_activate(move |app| {
        if let Err(error) = build_wallpaper(app, &url) {
            eprintln!("Could not create wallpaper surface: {error:#}");
        }
    });
    app.run();
    Ok(())
}

fn build_wallpaper(app: &Application, address: &str) -> Result<()> {
    let uri = if address.contains("://") {
        address.to_owned()
    } else {
        format!("https://{address}")
    };

    let webview = webkit::WebView::new();
    webview.load_uri(&uri);

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
