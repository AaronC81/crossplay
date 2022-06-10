#![feature(async_closure)]

use std::{sync::{Arc, RwLock}, marker::PhantomData};

use dioxus::prelude::*;
use library::Library;
use youtube::YouTubeDownload;

mod youtube;
mod library;

#[tokio::main]
async fn main() {
    console_subscriber::init();

    dioxus::desktop::launch(App)
}

#[derive(Props)]
struct SongDownloadProps<'a> {
    library: Arc<RwLock<Library>>,
    download: YouTubeDownload,

    // Seemingly required so that Dioxus' Props derive doesn't try to implement PartialEq, which
    // our `library` doesn't allow
    phantom: PhantomData<&'a ()>,
}

#[allow(non_snake_case)]
fn SongDownload<'a>(cx: Scope<'a, SongDownloadProps>) -> Element<'a> {
    let library = cx.props.library.clone();
    let download = cx.props.download.clone();
    let id = download.id.clone();
    let download_future = use_future(&cx, (), async move |_| {
        println!("Starting download future...");
        let library = library.read().unwrap();
        download.download(library).await.unwrap();
    });

    cx.render(match download_future.value() {
        Some(_) => rsx!(li { "Download complete! ({id})" }),
        None => rsx!(li { "Downloading video... ({id})" }),
    })
}

#[allow(non_snake_case)]
fn App(cx: Scope) -> Element {
    let mut library = Library::new("/Users/aaron/Music/CrossPlay".into());
    library.load_songs().unwrap();

    let library = Arc::new(RwLock::new(library));

    let library_ref = use_ref(&cx, || library.clone());

    cx.render(rsx!(
        h1 { "Songs" }
        button {
            onclick: move |_| { library_ref.write().write().unwrap().load_songs().unwrap(); },
            "Refresh"
        }
        ul {
            library.clone().read().unwrap().songs().map(|s| rsx!(li {
                b { "{s.metadata.title}" }
            }))
        }
        h1 { "Downloads" }
        ul {
            // TODO: doesn't refresh when a download finishes
            SongDownload {
                library: library,
                download: YouTubeDownload::new("rVJPAZ1Hxxk"),
                phantom: PhantomData
            }
        }
    ))
}
