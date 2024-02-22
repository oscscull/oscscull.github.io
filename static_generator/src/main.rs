use std::collections::HashMap;
use std::fs;
use std::fs::File;
use std::io;
use std::io::Write;
use std::path::Path;
use std::path::PathBuf;
use regex::Regex;
use walkdir::WalkDir;

fn main() -> io::Result<()> {
    println!("Current Working Directory: {:?}", std::env::current_dir());
    let src_dir = Path::new("./content");
    let dest_dir = Path::new("./docs");

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
    let templates = list_template_files("./templates")?;

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
                    let transcribed_content = transcribe(&base_file, &templates, HashMap::new());
                    write_string_to_file(&dest_file_path, &transcribed_content)?;
                    break;
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

fn transcribe(file: &str, templates: &HashMap<String, String>, variables: HashMap<String, String>) -> String {
    pretty_print_map(&variables);
    for (pattern, template) in templates {
        if let Some(captures) = Regex::new(pattern).unwrap().captures(&file) {
            let mut replaced_template = template.clone();
            for capture in captures.iter().skip(1) {
                if let Some(capture) = capture {
                    let placeholder = capture.as_str().to_string();
                    if let Some(replacement) = variables.get(&placeholder) {
                        replaced_template = replaced_template.replace(&format!("{{[{}]}}", placeholder), replacement);
                    }
                }
            }

            let replaced_file = Regex::new(pattern).unwrap().replace(&file, |caps: &regex::Captures| {
                let mut replaced = replaced_template.clone();
                for (_i, cap) in caps.iter().enumerate().skip(1) {
                    if let Some(placeholder) = cap {
                        if let Some(replacement) = variables.get(placeholder.as_str()) {
                            replaced = replaced.replace(&format!("{{[{}]}}", placeholder.as_str()), replacement);
                        }
                    }
                }
                replaced
            });

            return transcribe(&replaced_file, templates, variables);
        }
    }
    file.to_string()
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