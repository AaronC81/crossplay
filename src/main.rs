use dioxus::prelude::*;

mod youtube;
mod library;

#[tokio::main]
async fn main() {
    let lib = library::Library { path: "/tmp".into() };
    let dl = youtube::YouTubeDownload::new("rVJPAZ1Hxxk");
    dl.download(&lib).await.unwrap();

    dioxus::desktop::launch(App)
}

#[allow(non_snake_case)]
fn App(cx: Scope) -> Element {
    let mut count = use_state(&cx, || 0);

    cx.render(rsx!(
        h1 { "High-Five counter: {count}" }
        button { onclick: move |_| count += 1, "Up high!" }
        button { onclick: move |_| count -= 1, "Down low!" }
    ))
}
