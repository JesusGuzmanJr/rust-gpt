use maud::html;

pub fn index() -> impl IntoResponse {
    html! {
            html lang="en" {
                head {
                    title { "Rust GPT" }
                    link rel="stylesheet" href="/style/main.css"
                }
                body {
                    div.container {
                        h1 { "Rust GPT" }
                    }
                }
        }
    }
}
