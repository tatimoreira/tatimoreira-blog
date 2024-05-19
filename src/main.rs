use std::{fs};
mod templates;
fn main() {
    println!("Hello, world!");
    rebuild_site(content_dir, output_dir)
}

fn rebuild_site(content_dir: &str, output_dir: &str){
    let _ = fs::remove_dir_all(output_dir);


    let markdown_files: Vec<String> = walkdir::WalkDir::new(content_dir)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().display().to_string().ends_with(".md"))
        .map(|e| e.path().display().to_string())
        .collect();
    let  html_files: Vec<String>= Vec::with_capacity(markdown_files.len());

   for file in &markdown_files {
   println!("test")
   }
   


}