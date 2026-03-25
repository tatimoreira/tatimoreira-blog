use async_graphql::{Context, EmptyMutation, EmptySubscription, Object, Schema, SimpleObject};
use vercel_runtime::{run, Body, Error, Request, Response, StatusCode};

type BlogSchema = Schema<QueryRoot, EmptyMutation, EmptySubscription>;

#[derive(SimpleObject)]
struct Post {
    slug: String,
    title: String,
    body: String,
}

fn content_dir() -> String {
    std::env::var("CONTENT_DIR").unwrap_or_else(|_| "content".into())
}

fn find_content() -> Vec<String> {
    walkdir::WalkDir::new(content_dir())
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().display().to_string().ends_with(".md"))
        .map(|e| e.path().display().to_string())
        .collect()
}

fn load_post(file: &str) -> Option<Post> {
    let markdown = std::fs::read_to_string(file).ok()?;
    let parser = pulldown_cmark::Parser::new_ext(&markdown, pulldown_cmark::Options::all());
    let mut body = String::new();
    pulldown_cmark::html::push_html(&mut body, parser);

    let slug = std::path::Path::new(file)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_owned();

    let title = markdown
        .lines()
        .find(|l| l.starts_with("# "))
        .map(|l| l.trim_start_matches('#').trim().to_owned())
        .unwrap_or_else(|| slug.replace('-', " ").replace('_', " "));

    Some(Post { slug, title, body })
}

struct QueryRoot;

#[Object]
impl QueryRoot {
    async fn posts(&self, _ctx: &Context<'_>) -> Vec<Post> {
        find_content()
            .into_iter()
            .filter_map(|f| load_post(&f))
            .collect()
    }

    async fn post(&self, _ctx: &Context<'_>, slug: String) -> Option<Post> {
        let file = format!("{}/{}.md", content_dir(), slug);
        load_post(&file)
    }
}

fn build_schema() -> BlogSchema {
    Schema::build(QueryRoot, EmptyMutation, EmptySubscription).finish()
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    run(handler).await
}

pub async fn handler(req: Request) -> Result<Response<Body>, Error> {
    let schema = build_schema();

    if req.method().as_str() == "GET" {
        let html = async_graphql::http::playground_source(
            async_graphql::http::GraphQLPlaygroundConfig::new("/api/graphql"),
        );
        return Ok(Response::builder()
            .status(StatusCode::OK)
            .header("content-type", "text/html")
            .body(Body::Text(html))?);
    }

    let gql_req: async_graphql::Request = match req.body() {
        Body::Text(s) => serde_json::from_str(s)?,
        Body::Binary(b) => serde_json::from_slice(b)?,
        Body::Empty => return Err("empty request body".into()),
    };

    let gql_res = schema.execute(gql_req).await;

    Ok(Response::builder()
        .status(StatusCode::OK)
        .header("content-type", "application/json")
        .body(Body::Text(serde_json::to_string(&gql_res)?))?)
}
