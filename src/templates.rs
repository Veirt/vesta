use axum::response::Html;

pub async fn dashboard() -> Html<&'static str> {
    Html(
        r##"<!doctype html>
<html lang="en">
  <head>
    <meta charset="utf-8">
    <meta name="viewport" content="width=device-width, initial-scale=1">
    <meta name="color-scheme" content="light dark">
    <meta name="theme-color" content="#0a0a0a">
    <link rel="stylesheet" href="/static/ui/app.css">
    <script type="module" src="/static/ui/app.js"></script>
    <title>Vesta</title>
  </head>
  <body>
    <div id="root"></div>
  </body>
</html>"##,
    )
}
