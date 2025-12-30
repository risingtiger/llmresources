use anyhow::{Context, Result};
use console::style;
use dialoguer::{theme::ColorfulTheme, Confirm, Input, MultiSelect};
use skim::prelude::*;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
struct ConventionFile {
    name: String,
    path: PathBuf,
}

impl ConventionFile {
    fn new(path: PathBuf) -> Self {
        Self {
            name: path.file_name().unwrap_or_default().to_string_lossy().to_string(),
            path,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct Config {
    search_root: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            search_root: "/Users/dave/Code".to_string(),
        }
    }
}

fn main() -> Result<()> {
    print_banner();
    
    // 1. Find available convention files
    let convention_files = find_convention_files()?;
    if convention_files.is_empty() {
        println!("{}", style("No .md files found in conventions/ directory").red());
        return Ok(());
    }
    
    // 2. User selects which files to include
    let selected_files = select_convention_files(&convention_files)?;
    if selected_files.is_empty() {
        println!("{}", style("No files selected. Exiting.").yellow());
        return Ok(());
    }
    
    // 3. User selects target directory
    let target_dir = get_target_directory()?;
    
    // 4. Show summary and confirm
    show_summary(&selected_files, &target_dir);
    if !confirm_proceed()? {
        println!("{}", style("Operation cancelled.").yellow());
        return Ok(());
    }
    
    // 5. Generate the file
    generate_agents_file(&selected_files, &target_dir)?;
    
    Ok(())
}

fn print_banner() {
    println!("{}", style("Convention Compiler").blue().bold());
    println!("{}", style("===================").blue());
    println!();
}

fn find_convention_files() -> Result<Vec<ConventionFile>> {
    let conventions_dir = Path::new("conventions");
    if !conventions_dir.exists() {
        anyhow::bail!("conventions/ directory not found");
    }
    
    let mut files = Vec::new();
    for entry in fs::read_dir(conventions_dir)? {
        let path = entry?.path();
        if path.is_file() && path.extension().map_or(false, |ext| ext == "md") {
            files.push(ConventionFile::new(path));
        }
    }
    files.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(files)
}

fn select_convention_files(files: &[ConventionFile]) -> Result<Vec<ConventionFile>> {
    let file_names: Vec<&str> = files.iter().map(|f| f.name.as_str()).collect();
    
    let selections = MultiSelect::with_theme(&ColorfulTheme::default())
        .with_prompt("Select convention files to combine")
        .items(&file_names)
        .interact()?;
    
    Ok(selections.into_iter().map(|i| files[i].clone()).collect())
}

fn get_target_directory() -> Result<PathBuf> {
    let config = load_or_create_config()?;
    let search_root = PathBuf::from(&config.search_root);
    
    // Gather candidate directories
    let mut candidates = vec![PathBuf::from("."), PathBuf::from("..")];
    if search_root.exists() {
        candidates.push(search_root.clone());
        // Add immediate subdirectories of search root
        if let Ok(entries) = fs::read_dir(&search_root) {
            for entry in entries.flatten() {
                if entry.path().is_dir() {
                    candidates.push(entry.path());
                    
                    // Scan one level deeper for nested project directories (common in /Code)
                    if let Ok(sub_entries) = fs::read_dir(entry.path()) {
                        for sub in sub_entries.flatten().take(10) {
                            if sub.path().is_dir() {
                                candidates.push(sub.path());
                            }
                        }
                    }
                }
            }
        }
    }
    candidates.sort();
    candidates.dedup();

    // Format for display
    let mut display_items: Vec<String> = candidates.iter().map(|p| {
        let display = p.display().to_string();
        if display.starts_with(&config.search_root) {
            format!("📁 ~/{}", display.strip_prefix(&config.search_root).unwrap_or(&display).trim_start_matches('/'))
        } else {
            format!("📁 {}", display)
        }
    }).collect();
    display_items.push("✏️  Type custom path...".to_string());

    // Run Skim
    let options = SkimOptionsBuilder::default()
        .height(Some("40%"))
        .prompt(Some("Target > "))
        .reverse(true)
        .build()
        .map_err(|e| anyhow::anyhow!("Skim error: {}", e))?;

    let item_reader = SkimItemReader::default();
    let items = item_reader.of_bufread(std::io::Cursor::new(display_items.join("\n")));
    
    let selected_items = Skim::run_with(&options, Some(items))
        .ok_or_else(|| anyhow::anyhow!("No selection made"))?;

    if selected_items.is_abort {
        return Ok(PathBuf::from(".")); // Default to current on abort
    }

    let selected_text = selected_items.selected_items[0].output().to_string();
    
    if selected_text.contains("Type custom path") {
        let input: String = Input::with_theme(&ColorfulTheme::default())
            .with_prompt("Enter path")
            .interact_text()?;
        Ok(PathBuf::from(input))
    } else {
        // Map back to path
        let clean = selected_text.trim_start_matches("📁 ").trim();
        if clean.starts_with("~/") {
            Ok(search_root.join(clean.trim_start_matches("~/")))
        } else {
            Ok(PathBuf::from(clean))
        }
    }
}

fn show_summary(files: &[ConventionFile], target: &Path) {
    println!();
    println!("{}", style("Summary").cyan().bold());
    println!("{}", style("-------").cyan());
    println!("Input files:");
    for f in files {
        println!("  • {}", f.name);
    }
    println!();
    println!("Output: {}/AGENTS.md", style(target.display()).green());
    println!();
}

fn confirm_proceed() -> Result<bool> {
    Confirm::with_theme(&ColorfulTheme::default())
        .with_prompt("Create AGENTS.md?")
        .default(true)
        .interact()
        .context("Confirmation failed")
}

fn generate_agents_file(files: &[ConventionFile], target_dir: &Path) -> Result<()> {
    let mut content = String::new();
    
    for file in files {
        let file_content = fs::read_to_string(&file.path)
            .with_context(|| format!("Failed to read {}", file.name))?;
            
        content.push_str(&file_content);
        // Ensure clean separation
        if !content.ends_with('\n') { content.push('\n'); }
        if !content.ends_with("\n\n") { content.push('\n'); }
    }

    if !target_dir.exists() {
        println!("{}", style(format!("Creating directory: {}", target_dir.display())).yellow());
        fs::create_dir_all(target_dir)?;
    }

    let output_path = target_dir.join("AGENTS.md");
    fs::write(&output_path, content.trim())?;
    
    println!("{}", style("✓ AGENTS.md created successfully!").green().bold());
    Ok(())
}

fn load_or_create_config() -> Result<Config> {
    let path = Path::new("config.yaml");
    if path.exists() {
        let content = fs::read_to_string(path)?;
        Ok(serde_yaml::from_str(&content).unwrap_or_default())
    } else {
        let config = Config::default();
        fs::write(path, serde_yaml::to_string(&config)?)?;
        Ok(config)
    }
}
