use regex::Regex;
use std::collections::HashMap;
use std::fs;
use std::fs::File;
use std::io;
use std::io::Write;
use std::path::Path;
use std::path::PathBuf;
use walkdir::WalkDir;
use dotenv::dotenv;
use std::env;


fn main() -> io::Result<()> {
    dotenv().ok();
    println!("Current Working Directory: {:?}", std::env::current_dir());
    let src_dir_str = env::var("CONTENT_DIR").expect("Error: CONTENT_DIR environment variable not set");
    let src_dir = Path::new(&src_dir_str);

    // Get the destination directory path from the environment variable
    let dest_dir_str = env::var("DOCS_DIR").expect("Error: DOCS_DIR environment variable not set");
    let dest_dir = Path::new(&dest_dir_str);

    walk_files(src_dir, dest_dir)?;

    println!("HTML and CSS files copied successfully.");

    // List available templates
    //let templates_dir = "./templates";
    //let template_files = list_template_files(templates_dir)?;
    Ok(())
}

fn walk_files(src_dir: &Path, dest_dir: &Path) -> io::Result<()> {
    // Delete the destination directory if it exists
    if dest_dir.exists() {
        fs::remove_dir_all(dest_dir)?;
    }

    // Recreate the destination directory
    fs::create_dir_all(dest_dir)?;

    // List available templates
    let templates_dir_str = env::var("TEMPLATES_DIR").expect("Error: TEMPLATES_DIR environment variable not set");
    let templates = list_template_files(&templates_dir_str)?;

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
                        transcribe(&base_file, &templates, &preload_variables(), entry.path());
                    write_string_to_file(&dest_file_path, &transcribed_content)?;
                }
                if ext == "css" {
                    let relative_path = entry.path().strip_prefix(src_dir).unwrap();
                    let dest_file_path = dest_dir.join(relative_path);

                    // Create parent directories if they don't exist
                    if let Some(parent_dir) = dest_file_path.parent() {
                        fs::create_dir_all(parent_dir)?;
                    }

                    // Copy the CSS file
                    fs::copy(entry.path(), &dest_file_path)?;
                }
            }
        }
    }

    Ok(())
}

fn preload_variables() -> HashMap<String, String> {
    //recentText
    //recentDate
    //recentLink
    //recentTitle
    //recentImage
    //recentImageDescription
    let mut map = HashMap::new();

    let default_image_str = env::var("DEFAULT_IMAGE").expect("Error: DEFAULT_IMAGE environment variable not set");
    let default_image_desc_str = env::var("DEFAULT_IMAGE_DESC").expect("Error: DEFAULT_IMAGE_DESC environment variable not set");
    map.insert("recentImage".to_string(), default_image_str.to_string());
    map.insert("recentImageDescription".to_string(), default_image_desc_str.to_string());

    map
}

fn write_string_to_file(path: &PathBuf, content: &str) -> std::io::Result<()> {
    // Open the file for writing
    let mut file = File::create(path)?;

    // Write the string to the file
    file.write_all(content.as_bytes())?;

    Ok(())
}

fn pretty_print_map(map: &HashMap<String, String>) {
    for (key, value) in map {
        println!("{} => {}", key, value);
    }
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
            }
            '}' => {
                nesting_level -= 1;
                if nesting_level == 1 {
                    in_value = false;
                    result.insert(key.trim().to_string(), value.trim().to_string());
                    key.clear();
                    value.clear();
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
        result = result.replace(&format!("[[{}]]", placeholder), value);
    }

    // Replace any placeholders not found in the hashmap with an empty string
    let re = Regex::new(r"\\[\\[{}\\]\\]").unwrap();
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
                    return contents;
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
