use std::fs;
use std::io::{self, Write};

fn main() -> io::Result<()> {
    println!("Welcome to the HTML Template Editor!");
    // List available templates
    let templates_dir = "./templates";
    let template_files = fs::read_dir(templates_dir)?
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.file_type().map(|ft| ft.is_file()).unwrap_or(false))
        .map(|entry| entry.file_name().into_string().unwrap())
        .collect::<Vec<String>>();

    println!("Available Templates:");
    for (index, file_name) in template_files.iter().enumerate() {
        println!("{}. {}", index + 1, file_name);
    }

    // Ask user to select a template
    println!("Enter the number corresponding to the template you want to use:");
    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    let selected_template_index: usize = input.trim().parse().unwrap();
    if selected_template_index > 0 && selected_template_index <= template_files.len() {
        let selected_template_file = &template_files[selected_template_index - 1];

        // Load the selected template and create a corresponding text file with placeholders
        let template_content = fs::read_to_string(format!("{}/{}", templates_dir, selected_template_file))?;
        
        // Initialize a HashMap to store placeholders and their default content
    let mut placeholder_map: HashMap<String, String> = HashMap::new();

    // Extract placeholder tokens from the HTML template and store them in the HashMap
    for line in template_content.lines() {
        for token in line.split("$$").skip(1).step_by(2) {
            placeholder_map.insert(token.to_string(), String::from("your content here"));
        }
    }


        // Wait for user input to proceed
        println!("Press Enter when you have finished editing the text file.");
        io::stdin().read_line(&mut input)?;

        // Generate the final HTML content by replacing placeholders with user input
        let user_input_content = fs::read_to_string("user_input.txt")?;
        let mut final_html_content = String::new();

        // Write the final HTML content to a new HTML file
        let mut output_html_file = fs::File::create("output.html")?;
        write!(output_html_file, "{}", final_html_content)?;
    } else {
        println!("Invalid selection.");
    }

    Ok(())
}
