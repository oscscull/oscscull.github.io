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

                    // Copy the file
                    fs::copy(entry.path(), &dest_file_path)?;
                }
                if ext == "html" || ext == "css" {
                    let relative_path = entry.path().strip_prefix(src_dir).unwrap();
                    let dest_file_path = dest_dir.join(relative_path);

                    // Create parent directories if they don't exist
                    if let Some(parent_dir) = dest_file_path.parent() {
                        fs::create_dir_all(parent_dir)?;
                    }

                    if ext == "css" {
                        fs::copy(entry.path(), &dest_file_path)?;
                    }

                    if ext == "html" {
                        let transcribed_file = transcribe(entry.path())?;
                        write_string_to_file(&dest_file_path, &transcribed_file)?;
                    }
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

fn transcribe(file_path: &Path) -> io::Result<String> {
    // Read the contents of the file into a string
    let file_content = fs::read_to_string(file_path)?;

    //examples of tag and attribute regexes
    let page_tag_regex = Regex::new(r"<page>(.*?)</page>").unwrap();
    let attribute_regex = Regex::new(r"(\w+)={{(.*?)}}").unwrap();

    /*
    first modify list_template_files to get a hashmap 
    like {
        regex => file contents
    }
    for all the templates!

    then write the following algo:
    for each template perform the first replace you can, putting any variables captured into the hashmap
    then if you did a replace, start over again
    any time you hit a variable entry, get it from the hash map.
    then do markup at the end!
    */


    //Probably won't use the below as written

    // Perform substitutions
    let modified_content = page_tag_regex.replace_all(&file_content, "replacement1");
    // Return the modified content as a string
    Ok(modified_content.into_owned())
}

fn list_template_files(templates_dir: &str) -> io::Result<Vec<String>> {
    let template_files = fs::read_dir(templates_dir)?
        .filter_map(|entry| {
            if let Ok(entry) = entry {
                if let Ok(ft) = entry.file_type() {
                    if ft.is_file() {
                        let file_name = entry.file_name();
                        let file_name = file_name.to_string_lossy().to_string();
                        let file_stem = Path::new(&file_name)
                            .file_stem()
                            .and_then(|s| s.to_str())
                            .map(|s| s.to_string());
                        return file_stem;
                    }
                }
            }
            None
        })
        .collect::<Vec<String>>();

    Ok(template_files)
}
