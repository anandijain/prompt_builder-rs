use clap::{Parser, Subcommand};
use glob::Pattern;
use std::error::Error;
use std::fs::{self, File};
use std::io::{self, BufRead, BufReader, Write};
use std::path::Path;
use tiktoken_rs::{p50k_base};

/// A command-line utility for processing files in a directory
#[derive(Parser)]
#[command(name = "prompt_builder")]
#[command(about = "Processes files in a directory", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Tokenize contents of files in the specified directory and display token counts
    TokenizeDir {
        /// Path to the target directory
        directory: String,

        /// Specify output file path. If not provided, outputs to stdout
        #[arg(short, long, value_name = "FILE")]
        output: Option<String>,

        /// Ignore files matching the given glob pattern. Can be used multiple times
        #[arg(short, long, value_name = "GLOB")]
        ignore: Vec<String>,

        /// Skip lines containing the specified substring. Can be used multiple times
        #[arg(short, long, value_name = "SUBSTRING")]
        skip: Vec<String>,
    },
    /// Build prompts from file names and their contents
    DirPrompt {
        /// Path to the target directory
        directory: String,

        /// Specify output file path. If not provided, outputs to stdout
        #[arg(short, long, value_name = "FILE")]
        output: Option<String>,

        /// Ignore files matching the given glob pattern. Can be used multiple times
        #[arg(short, long, value_name = "GLOB")]
        ignore: Vec<String>,

        /// Skip lines containing the specified substring. Can be used multiple times
        #[arg(short, long, value_name = "SUBSTRING")]
        skip: Vec<String>,
    },
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::TokenizeDir {
            directory,
            output,
            ignore,
            skip,
        } => {
            if let Err(e) = tokenize_directory(&directory, output, ignore, skip) {
                eprintln!("Error: {}", e);
                std::process::exit(1);
            }
        }
        Commands::DirPrompt {
            directory,
            output,
            ignore,
            skip,
        } => {
            if let Err(e) = build_prompt_directory(&directory, output, ignore, skip) {
                eprintln!("Error: {}", e);
                std::process::exit(1);
            }
        }
    }
}

/// Tokenizes contents of files in the given directory and prints token counts
fn tokenize_directory(
    dir_path: &str,
    output: Option<String>,
    ignore_patterns: Vec<String>,
    skip_substrings: Vec<String>,
) -> Result<(), Box<dyn Error>> {
    let path = Path::new(dir_path);
    if !path.is_dir() {
        return Err(format!("{} is not a directory.", dir_path).into());
    }

    // Compile ignore patterns into glob::Pattern
    let ignore_globs: Vec<Pattern> = ignore_patterns
        .iter()
        .map(|p| Pattern::new(p).unwrap_or_else(|_| {
            eprintln!("Warning: Invalid ignore pattern '{}'. Ignoring.", p);
            // Return a pattern that matches nothing
            Pattern::new("a^").unwrap()
        }))
        .collect();

    // Prepare the output: either a file or stdout
    let mut writer: Box<dyn Write> = match output {
        Some(ref file_path) => {
            let file = File::create(file_path).map_err(|e| {
                format!(
                    "Failed to create output file '{}': {}",
                    file_path, e
                )
            })?;
            Box::new(file)
        }
        None => Box::new(io::stdout()),
    };

    // Initialize the tokenizer with the appropriate encoding
    let encoding = p50k_base()?;

    // Iterate over each entry in the directory
    for entry in fs::read_dir(path)? {
        let entry = entry?;
        let file_path = entry.path();

        if file_path.is_file() {
            let file_name_os = entry.file_name();
            let file_name_str = file_name_os.to_string_lossy();

            // Check if file matches any ignore pattern
            if ignore_globs.iter().any(|p| p.matches(&file_name_str)) {
                println!("Skipping ignored file: {}", file_name_str);
                continue;
            }

            // Read file contents
            let contents = fs::read_to_string(&file_path).unwrap_or_else(|_| {
                eprintln!("Warning: Could not read file {}", file_path.display());
                String::from("[Could not read contents]")
            });

            // Process contents: skip lines containing any of the specified substrings
            let processed_contents = if skip_substrings.is_empty() {
                contents
            } else {
                contents
                    .lines()
                    .filter(|line| {
                        !skip_substrings.iter().any(|substr| line.contains(substr))
                    })
                    .collect::<Vec<&str>>()
                    .join("\n")
            };

            // Tokenize the processed contents
            let tokens = encoding.encode_with_special_tokens(&processed_contents);
            let token_count = tokens.len();

            // Write to output
            writeln!(writer, "{}   {} tokens", file_name_str, token_count)?;
        }
    }

    Ok(())
}

/// Builds prompts from file names and their contents with additional options
fn build_prompt_directory(
    dir_path: &str,
    output: Option<String>,
    ignore_patterns: Vec<String>,
    skip_substrings: Vec<String>,
) -> Result<(), Box<dyn Error>> {
    let path = Path::new(dir_path);
    if !path.is_dir() {
        return Err(format!("{} is not a directory.", dir_path).into());
    }

    // Compile ignore patterns into glob::Pattern
    let ignore_globs: Vec<Pattern> = ignore_patterns
        .iter()
        .map(|p| Pattern::new(p).unwrap_or_else(|_| {
            eprintln!("Warning: Invalid ignore pattern '{}'. Ignoring.", p);
            // Return a pattern that matches nothing
            Pattern::new("a^").unwrap()
        }))
        .collect();

    // Prepare the output: either a file or stdout
    let mut writer: Box<dyn Write> = match output {
        Some(ref file_path) => {
            let file = File::create(file_path).map_err(|e| {
                format!(
                    "Failed to create output file '{}': {}",
                    file_path, e
                )
            })?;
            Box::new(file)
        }
        None => Box::new(io::stdout()),
    };

    // Iterate over each entry in the directory
    for entry in fs::read_dir(path)? {
        let entry = entry?;
        let file_path = entry.path();

        if file_path.is_file() {
            let file_name_os = entry.file_name();
            let file_name_str = file_name_os.to_string_lossy();

            // Check if file matches any ignore pattern
            if ignore_globs.iter().any(|p| p.matches(&file_name_str)) {
                println!("Skipping ignored file: {}", file_name_str);
                continue;
            }

            // Read file contents
            let contents = fs::read_to_string(&file_path).unwrap_or_else(|_| {
                eprintln!("Warning: Could not read file {}", file_path.display());
                String::from("[Could not read contents]")
            });

            // Process contents: skip lines containing any of the specified substrings
            let processed_contents = if skip_substrings.is_empty() {
                contents
            } else {
                contents
                    .lines()
                    .filter(|line| {
                        !skip_substrings.iter().any(|substr| line.contains(substr))
                    })
                    .collect::<Vec<&str>>()
                    .join("\n")
            };

            // Write to output
            writeln!(writer, "{}\n\n{}\n", file_name_str, processed_contents)?;
        }
    }

    Ok(())
}
