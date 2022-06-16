#![feature(async_closure)]

use std::{sync::{Arc, RwLock}, marker::PhantomData, borrow::BorrowMut};

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

#[derive(Props)]
struct SongDownloadPanelProps<'a> {
    library: Arc<RwLock<Library>>,
    phantom: PhantomData<&'a ()>,
}

#[allow(non_snake_case)]
fn SongDownloadPanel<'a>(cx: Scope<'a, SongDownloadPanelProps>) -> Element<'a> {
    let library = cx.props.library.clone();

    let id_input = use_state(&cx, || "".to_string());
    let downloads = use_ref(&cx, || vec![]);

    // Hack: Can't clone `library` in the map because it gets moved, so build a list of downloads
    // with libraries to use here 
    let mut downloads_plus_libraries = vec![];
    let downloads_values = downloads.read();
    for download in downloads_values.iter() {
        downloads_plus_libraries.push((download, library.clone()));
    }

    cx.render(rsx!(
        downloads_plus_libraries.iter().map(|(d, l): &(&YouTubeDownload, Arc<_>)| rsx!(
            SongDownload {
                library: l.clone(),
                download: (*d).clone(),
                phantom: PhantomData,
            }
        ))

        input {
            value: "{id_input}",
            oninput: move |evt| id_input.set(evt.value.clone()),
        }

        button {
            onclick: move |_| downloads.write().push(YouTubeDownload::new(id_input.get())),
            "Download"
        }
    ))
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
            SongDownloadPanel {
                library: library,
                phantom: PhantomData,
            }
            // SongDownload {
            //     library: library,
            //     download: YouTubeDownload::new("rVJPAZ1Hxxk"),
            //     phantom: PhantomData
            // }
        }
    ))
}
