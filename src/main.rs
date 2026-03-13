use axum::{extract::Extension, response::Html, routing::post, Json, Router};
use std::{fs, net::SocketAddr, path::Path, thread, time::Duration};
use tera::Tera;
use tower_http::services::ServeDir;
#[macro_use]
extern crate lazy_static;
extern crate tera;

mod graphql;

lazy_static! {
    pub static ref TEMPLATES: Tera = parse_tera();
}

fn parse_tera() -> Tera {
    match Tera::new("templates/**/*.html") {
        Ok(tera) => tera,
        Err(e) => {
            println!("Parsing error(s): {}", e);
            ::std::process::exit(1);
        }
    }
}

const CONTENT_DIR: &str = "content";
const PUBLIC_DIR: &str = "public";

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    let files = find_content(CONTENT_DIR);

    for file in files {
        println!("{:?}", file);
    }

    let _ = generate_site(CONTENT_DIR, PUBLIC_DIR);

    tokio::task::spawn_blocking(move || {
        println!("listenning for changes: {}", CONTENT_DIR);
        let mut hotwatch = hotwatch::Hotwatch::new().expect("hotwatch failed to initialize!");
        hotwatch
            .watch(CONTENT_DIR, |_| {
                println!("Rebuilding site");
                generate_site(CONTENT_DIR, PUBLIC_DIR).expect("Rebuilding site");
            })
            .expect("failed to watch content folder!");
        loop {
            thread::sleep(Duration::from_secs(1));
        }
    });

    let schema = graphql::build_schema();

    let app = Router::new()
        .route("/graphql", post(graphql_handler).get(graphiql_handler))
        .layer(Extension(schema))
        .nest_service("/", ServeDir::new(PUBLIC_DIR));

    let port: u16 = std::env::var("PORT")
        .unwrap_or_else(|_| "3001".into())
        .parse()
        .unwrap_or(3001);
    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    println!("serving site on {}", addr);
    println!("graphql endpoint: http://{}/graphql", addr);

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app.into_make_service())
        .await
        .unwrap();

    Ok(())
}

async fn graphiql_handler() -> Html<String> {
    Html(async_graphql::http::playground_source(
        async_graphql::http::GraphQLPlaygroundConfig::new("/graphql"),
    ))
}

async fn graphql_handler(
    Extension(schema): Extension<graphql::BlogSchema>,
    Json(req): Json<async_graphql::Request>,
) -> Json<async_graphql::Response> {
    Json(schema.execute(req).await)
}

fn find_content(content_dir: &str) -> Vec<String> {
    walkdir::WalkDir::new(content_dir)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().display().to_string().ends_with(".md"))
        .map(|e| e.path().display().to_string())
        .collect()
}

fn generate_site(content_dir: &str, output_dir: &str) -> Result<(), anyhow::Error> {
    let _ = fs::remove_dir_all(output_dir);

    let markdown_files: Vec<String> = find_content(content_dir);
    let mut html_files = Vec::with_capacity(markdown_files.len());

    for file in &markdown_files {
        let markdown = fs::read_to_string(&file)?;
        let parser = pulldown_cmark::Parser::new_ext(&markdown, pulldown_cmark::Options::all());

        let mut body = String::new();
        pulldown_cmark::html::push_html(&mut body, parser);

        let title = Path::new(&file)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_owned();

        let mut context = tera::Context::new();
        context.insert("body", &body);
        context.insert("title", &title);

        let html = TEMPLATES.render("post.html", &context)?;

        let html_file = file
            .replace(content_dir, output_dir)
            .replace(".md", ".html");
        let folder = Path::new(&html_file).parent().unwrap();
        let _ = fs::create_dir_all(folder);
        fs::write(&html_file, html)?;

        html_files.push(html_file);
    }

    write_index(html_files, output_dir)?;
    Ok(())
}

fn write_index(files: Vec<String>, output_dir: &str) -> Result<(), anyhow::Error> {
    let posts: Vec<serde_json::Value> = files
        .into_iter()
        .map(|file| {
            let file = file.trim_start_matches(output_dir).to_owned();
            let title = file.trim_start_matches('/').trim_end_matches(".html").to_owned();
            serde_json::json!({ "file_name": file, "title": title })
        })
        .collect();

    let mut context = tera::Context::new();
    context.insert("posts", &posts);

    let html = TEMPLATES.render("home.html", &context)?;

    let index_path = Path::new(&output_dir).join("index.html");
    fs::write(index_path, html)?;
    Ok(())
}

/*async fn handle_error(_err: io::Error) -> impl IntoResponse {
    (StatusCode::INTERNAL_SERVER_ERROR, "Something went wrong...")
}*/
