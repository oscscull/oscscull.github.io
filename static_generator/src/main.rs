use chrono::NaiveDateTime;
use dotenv::dotenv;
use fancy_regex::Regex;
use markdown::to_html;
use minify_html::minify;
use minify_html::Cfg;
use std::collections::HashMap;
use std::env;
use std::fs;
use std::fs::File;
use std::io;
use std::io::Write;
use std::path::Path;
use std::path::PathBuf;
use walkdir::WalkDir;

fn main() -> io::Result<()> {
    dotenv().ok();
    println!("Current Working Directory: {:?}", std::env::current_dir());
    let src_dir_str =
        env::var("CONTENT_DIR").expect("Error: CONTENT_DIR environment variable not set");
    let src_dir = Path::new(&src_dir_str);

    // Get the destination directory path from the environment variable
    let dest_dir_str = env::var("DOCS_DIR").expect("Error: DOCS_DIR environment variable not set");
    let dest_dir = Path::new(&dest_dir_str);

    walk_files(src_dir, dest_dir)?;

    println!("HTML and CSS files copied successfully.");

    Ok(())
}

fn walk_files(src_dir: &Path, dest_dir: &Path) -> io::Result<()> {
    // Delete the destination directory if it exists
    if dest_dir.exists() {
        fs::remove_dir_all(dest_dir)?;
    }

    // Recreate the destination directory
    fs::create_dir_all(dest_dir)?;

    // Write the CNAME if needed
    let cname_path = Path::new(dest_dir).join("CNAME");
    if let Ok(cname_value) = env::var("CNAME") {
        fs::write(&cname_path, cname_value)?;
    }

    // List available templates
    let templates_dir_str =
        env::var("TEMPLATES_DIR").expect("Error: TEMPLATES_DIR environment variable not set");
    let templates = list_template_files(&templates_dir_str)?;
    let variables = preload_variables()?;

    // Copy files from source to destination
    for entry in WalkDir::new(src_dir).into_iter().filter_map(|e| e.ok()) {
        let file_type = entry.file_type();

        if file_type.is_file() {
            let file_extension = entry.path().extension().and_then(|e| e.to_str());
            if let Some(ext) = file_extension {
                if ext == "html" {
                    let relative_path = entry.path().strip_prefix(src_dir).unwrap();
                    let dest_file_path = dest_dir.join(relative_path);

                    // Create parent directories if they don't exist
                    if let Some(parent_dir) = dest_file_path.parent() {
                        fs::create_dir_all(parent_dir)?;
                    }

                    let base_file = fs::read_to_string(entry.path())?;
                    let transcribed_content =
                        transcribe(&base_file, &templates, &variables, entry.path());
                    minified_string_to_file(&dest_file_path, &transcribed_content)?;
                }
                if ext == "css" {
                    let relative_path = entry.path().strip_prefix(src_dir).unwrap();
                    let dest_file_path = dest_dir.join(relative_path);

                    // Create parent directories if they don't exist
                    if let Some(parent_dir) = dest_file_path.parent() {
                        fs::create_dir_all(parent_dir)?;
                    }

                    let transcribed_content = fs::read_to_string(entry.path())?;
                    // Copy the CSS file
                    minified_string_to_file(&dest_file_path, &transcribed_content)?;
                }
            }
        }
    }

    Ok(())
}

fn preload_variables() -> Result<HashMap<String, String>, std::io::Error> {
    let mut map = HashMap::new();

    let default_image_str =
        env::var("DEFAULT_IMAGE").expect("Error: DEFAULT_IMAGE environment variable not set");
    let default_image_desc_str = env::var("DEFAULT_IMAGE_DESC")
        .expect("Error: DEFAULT_IMAGE_DESC environment variable not set");
    map.insert("recentImage".to_string(), default_image_str.to_string());
    map.insert(
        "recentImageDescription".to_string(),
        default_image_desc_str.to_string(),
    );

    let default_content_dir =
        env::var("CONTENT_DIR").expect("Error: CONTENT_DIR environment variable not set");
    let default_article_ext =
        env::var("ARTICLES_SUBDIR").expect("Error: ARTICLES_SUBDIR environment variable not set");
    let (recent_article_path, recent_article_date) =
        most_recent_html_file(&(default_content_dir.to_string() + &default_article_ext));
    map.insert(
        "recentDate".to_string(),
        recent_article_date.format("%Y-%m-%d %H:%M").to_string(),
    );

    let relative_path = recent_article_path
        .strip_prefix(default_content_dir)
        .unwrap();
    map.insert("recentLink".to_string(), path_to_html_path(&relative_path));

    let mut article_file = fs::read_to_string(&recent_article_path)?;

    let re = Regex::new(r#"<(.*)>((.|\n)*)</\1>"#).unwrap();
    if let Ok(Some(captures)) = re.captures(&article_file) {
        if let Some(capture) = captures.get(2) {
            article_file = capture.as_str().to_string();
        }
    }
    article_file = replace_md_placeholder(&article_file, &recent_article_path);
    let vars = read_vars(&article_file);

    if let Some(value) = vars.get("title") {
        map.insert("recentTitle".to_string(), value.to_string());
    }

    if let Some(value) = vars.get("content") {
        map.insert("recentText".to_string(), short_text_preview(value.to_string()));
    }

    if let Some(value) = vars.get("imageHero") {
        map.insert("recentImage".to_string(), value.to_string());
    }

    if let Some(value) = vars.get("imageHeroAlt") {
        map.insert("recentImageDescription".to_string(), value.to_string());
    }

    map.insert("articleList".to_string(), load_articles()?);
    Ok(map)
}

fn load_articles() -> io::Result<String> {
    let mut final_string = String::from("");
    let default_content_dir =
        env::var("CONTENT_DIR").expect("Error: CONTENT_DIR environment variable not set");
    let default_article_ext =
        env::var("ARTICLES_SUBDIR").expect("Error: ARTICLES_SUBDIR environment variable not set");
    let templates_dir_str =
        env::var("TEMPLATES_DIR").expect("Error: TEMPLATES_DIR environment variable not set");
    let default_pages_item_template = env::var("PAGES_ITEM_TEMPLATE")
        .expect("Error: PAGES_ITEM_TEMPLATE environment variable not set");

    let template =
        fs::read_to_string(&(templates_dir_str.to_string() + "/" + &default_pages_item_template))?;
    let paths = fs::read_dir(&(default_content_dir.to_string() + &default_article_ext))?;

    for path in paths {
        let entry = path?;
        let file_path = entry.path();

        if file_path.is_file() {
            if let Some(extension) = file_path.extension() {
                if extension == "html" {
                    let mut map = HashMap::new();
                    let re = Regex::new(r#"date=\{\{(.*)\}\}"#).unwrap();
                    let article_file = fs::read_to_string(&file_path)?;
                    if let Ok(Some(captures)) = re.captures(&article_file) {
                        if let Some(capture) = captures.get(1) {
                            map.insert("itemDate".to_string(), capture.as_str().to_string());
                        }
                    }
                    let re = Regex::new(r#"title=\{\{(.*)\}\}"#).unwrap();
                    if let Ok(Some(captures)) = re.captures(&article_file) {
                        if let Some(capture) = captures.get(1) {
                            map.insert(
                                "itemLink".to_string(),
                                default_article_ext.to_string()
                                    + "/"
                                    + &file_path.file_name().unwrap().to_string_lossy().to_string(),
                            );
                            map.insert("itemDescription".to_string(), capture.as_str().to_string());
                            final_string.push_str(&replace_vars(&template, &map))
                        }
                    }
                }
            }
        }
    }

    Ok(final_string.to_string())
}

fn most_recent_html_file(directory_path: &str) -> (PathBuf, NaiveDateTime) {
    let mut most_recent_html_file: Option<(PathBuf, NaiveDateTime)> = None;

    if let Ok(entries) = fs::read_dir(directory_path) {
        for entry in entries {
            if let Ok(entry) = entry {
                if let Some(extension) = entry.path().extension() {
                    if extension == "html" {
                        if let Some(_file_name) = entry.file_name().to_str() {
                            if let Ok(article_file) = fs::read_to_string(&entry.path()) {
                                let re = Regex::new(r#"date={{(.*)}}"#).unwrap();
                                if let Ok(Some(captures)) = re.captures(&article_file) {
                                    if let Some(capture) = captures.get(1) {
                                        let naive_datetime = NaiveDateTime::parse_from_str(
                                            capture.as_str(),
                                            "%Y-%m-%d %H:%M",
                                        )
                                        .unwrap();
                                        if let Some((_, most_recent_time)) = most_recent_html_file {
                                            if naive_datetime > most_recent_time {
                                                most_recent_html_file =
                                                    Some((entry.path(), naive_datetime));
                                            }
                                        } else {
                                            most_recent_html_file =
                                                Some((entry.path(), naive_datetime));
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    } else {
        eprintln!("Error reading directory.");
    }

    // Return the path of the most recent HTML file, if any
    if let Some((path, datetime)) = most_recent_html_file {
        return (path, datetime);
    }

    // If no HTML file found, panic
    panic!("No HTML file with date found in the articles directory.");
}

fn path_to_html_path(path: &Path) -> String {
    let mut html_path = String::new();

    html_path.push('/');

    for component in path.components() {
        let part = component.as_os_str().to_string_lossy();
        html_path.push_str(&part);
        html_path.push('/');
    }

    // Remove trailing slash if the path is not empty
    if !html_path.is_empty() {
        html_path.pop();
    }

    html_path
}

fn minified_string_to_file(path: &PathBuf, content: &str) -> std::io::Result<()> {
    let mut file = File::create(path)?;
    let mut bytes_vec: Vec<u8> = content.as_bytes().to_vec();
    let cfg = Cfg::new();
    // Get the command-line arguments
    let args: Vec<String> = env::args().collect();

    // Check if the specific flag is passed
    if !args.iter().any(|arg| arg == "--nominify") {
        bytes_vec = minify(&bytes_vec, &cfg);
    }

    file.write_all(&bytes_vec)?;

    Ok(())
}

fn transcribe(
    file: &str,
    templates: &HashMap<String, String>,
    variables: &HashMap<String, String>,
    parent: &Path,
) -> String {
    let mut final_result = file.to_string();
    final_result = replace_vars(&final_result, variables);
    final_result = replace_md_placeholder(&final_result, parent);
    let mut extracted_variables = variables.clone();

    loop {
        let mut found_match = false;

        for (pattern, template) in templates {
            if let Some(captures) = regex::Regex::new(pattern).unwrap().captures(&final_result) {
                found_match = true;
                let mut replaced_template = template.clone();
                for capture in captures.iter().skip(1) {
                    if let Some(capture) = capture {
                        let placeholder = capture.as_str().to_string();
                        extracted_variables.extend(read_vars(&placeholder));
                    }
                }
                replaced_template =
                    transcribe(&replaced_template, templates, &extracted_variables, parent);
                let re = Regex::new(pattern).unwrap();
                final_result = re.replace(&final_result, replaced_template).to_string();
            }
        }

        if !found_match {
            // No more matches found, break out of the loop
            break;
        }
    }

    final_result
}

fn read_vars(input: &str) -> HashMap<String, String> {
    let mut result = HashMap::new();
    let mut key = String::new();
    let mut value = String::new();
    let mut nesting_level = 0;
    let mut in_value = false;

    for c in input.chars() {
        match c {
            '{' => {
                nesting_level += 1;
                if nesting_level == 2 {
                    in_value = true;
                }
                if nesting_level > 2 {
                    value.push(c);
                }
            }
            '}' => {
                nesting_level -= 1;
                if nesting_level == 1 {
                    in_value = false;
                    result.insert(key.trim().to_string(), value.trim().to_string());
                    key.clear();
                    value.clear();
                }
                if nesting_level > 2 {
                    value.push(c);
                }
            }
            _ => {
                if in_value {
                    value.push(c);
                } else {
                    if c != '=' {
                        key.push(c);
                    }
                }
            }
        }
    }
    result
}

fn replace_vars(string: &str, replacements: &HashMap<String, String>) -> String {
    let mut result = string.to_string();
    for (placeholder, value) in replacements {
        let pattern = format!(r"\[\[{}]]", regex::escape(placeholder));
        let re = Regex::new(&pattern).unwrap();
        result = re.replace_all(&result, value.as_str()).to_string();
    }
    
    // Replace any placeholders not found in the hashmap with an empty string
    let re = Regex::new(r"\[\[.*\]\]").unwrap();
    result = re.replace_all(&result, "").to_string();
    
    result
}

fn replace_md_placeholder(input: &str, file_path: &Path) -> String {
    let re = regex::Regex::new(r#"<md .*src="(.*)">.*</md>"#).unwrap();
    let file_contents = re.replace_all(input, |caps: &regex::Captures| {
        if let Some(filename) = caps.get(1).map(|m| m.as_str()) {
            if let Some(parent_dir) = file_path.parent() {
                let full_path = PathBuf::from(parent_dir).join(filename);
                if let Ok(contents) = fs::read_to_string(&full_path) {
                    return to_html(&contents);
                }
            }
        }
        "".to_string() // Return empty string if the file couldn't be read or if capture group is invalid
    });

    file_contents.to_string()
}

fn list_template_files(templates_dir: &str) -> io::Result<HashMap<String, String>> {
    let mut template_files = HashMap::new();

    for entry in fs::read_dir(templates_dir)? {
        if let Ok(entry) = entry {
            if let Ok(ft) = entry.file_type() {
                if ft.is_file() {
                    let file_name = entry.file_name().to_string_lossy().to_string();
                    let file_stem = Path::new(&file_name)
                        .file_stem()
                        .and_then(|s| s.to_str())
                        .map(|s| s.to_string());

                    if let Some(stem) = file_stem {
                        if let Ok(content) = fs::read_to_string(entry.path()) {
                            let pattern = format!("<{}>((.|\n)*?)</{}>", stem, stem);
                            template_files.insert(pattern, content);
                        }
                    }
                }
            }
        }
    }

    Ok(template_files)
}

fn short_text_preview(input: String) -> String
{
    // Remove all HTML tags
    let input_no_tags = Regex::new(r#"<[^>]+>"#)
        .unwrap()
        .replace_all(&input, "");

    // Trim to 15 characters and add ellipsis if necessary
    let trimmed_text = if input_no_tags.chars().count() > 75 {
        input_no_tags.chars().take(75).collect::<String>() + "..."
    } else {
        input_no_tags.to_string()
    };

    trimmed_text
}
