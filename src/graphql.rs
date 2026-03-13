use async_graphql::{Context, Object, Schema, SimpleObject};

use crate::{find_content, CONTENT_DIR};

pub type BlogSchema = Schema<QueryRoot, async_graphql::EmptyMutation, async_graphql::EmptySubscription>;

#[derive(SimpleObject)]
pub struct Post {
    pub slug: String,
    pub title: String,
    pub body: String,
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

    let title = slug.replace('-', " ").replace('_', " ");

    Some(Post { slug, title, body })
}

pub struct QueryRoot;

#[Object]
impl QueryRoot {
    async fn posts(&self, _ctx: &Context<'_>) -> Vec<Post> {
        find_content(CONTENT_DIR)
            .into_iter()
            .filter_map(|f| load_post(&f))
            .collect()
    }

    async fn post(&self, _ctx: &Context<'_>, slug: String) -> Option<Post> {
        let file = format!("{}/{}.md", CONTENT_DIR, slug);
        load_post(&file)
    }
}

pub fn build_schema() -> BlogSchema {
    Schema::build(QueryRoot, async_graphql::EmptyMutation, async_graphql::EmptySubscription)
        .finish()
}
