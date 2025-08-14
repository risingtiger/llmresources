use anyhow::{Context, Result};
use console::style;
use dialoguer::{theme::ColorfulTheme, Confirm, Input, MultiSelect, Select};
use skim::prelude::*;
use serde::{Deserialize, Serialize};
use std::fs;
use std::os::unix::fs as unix_fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
struct ConventionFile {
    name: String,
    path: PathBuf,
}

impl ConventionFile {
    fn new(path: PathBuf) -> Self {
        let name = path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();
        
        Self { name, path }
    }
    
}

#[derive(Debug, Serialize, Deserialize)]
struct Config {
    search_root: String,
    fuzzy_search: FuzzySearchConfig,
}

#[derive(Debug, Serialize, Deserialize)]
struct FuzzySearchConfig {
    show_hidden: bool,
    max_depth: u8,
    prompt: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            search_root: "/Users/dave/Code".to_string(),
            fuzzy_search: FuzzySearchConfig {
                show_hidden: false,
                max_depth: 2,
                prompt: "Select directory > ".to_string(),
            },
        }
    }
}

fn main() -> Result<()> {
    print_banner();
    
    let command = select_command()
        .context("Failed to select command")?;
    
    match command.as_str() {
        "symlink" => run_compile_command()?,
        _ => {
            println!("{}", style("Unknown command").red());
            return Ok(());
        }
    }
    
    Ok(())
}

fn print_banner() {
    println!("{}", style("Convention Markdown Compiler").blue().bold());
    println!("{}", style("==================================").blue());
    println!();
}

fn select_command() -> Result<String> {
    let commands = vec![
        "symlink - Create symlinks for convention markdown files",
    ];
    
    println!("{}", style("Available Commands:").green().bold());
    println!();
    
    let selection = Select::with_theme(&ColorfulTheme::default())
        .with_prompt("Select a command")
        .items(&commands)
        .default(0)
        .interact()
        .context("Failed to get command selection")?;
    
    // Extract just the command name (before the " - " description)
    let command = commands[selection].split(" - ").next().unwrap_or("symlink");
    
    println!();
    Ok(command.to_string())
}

fn run_compile_command() -> Result<()> {
    let convention_files = find_convention_files()
        .context("Failed to find convention files")?;
    
    if convention_files.is_empty() {
        println!("{}", style("No .md files found in conventions/ directory").red());
        return Ok(());
    }
    
    let selected_files = select_convention_files(&convention_files)
        .context("Failed to select convention files")?;
    
    if selected_files.is_empty() {
        println!("{}", style("No files selected. Exiting.").yellow());
        return Ok(());
    }
    
    let target_dir = get_target_directory()
        .context("Failed to get target directory")?;
    
    show_summary(&selected_files, &target_dir)?;
    
    if !confirm_proceed()? {
        println!("{}", style("Operation cancelled.").yellow());
        return Ok(());
    }
    
    compile_files(&selected_files, &target_dir)
        .context("Failed to create symlinks")?;
    
    println!("{}", style("✓ Successfully created symlinks for convention files!").green().bold());
    
    Ok(())
}

fn find_convention_files() -> Result<Vec<ConventionFile>> {
    let conventions_dir = Path::new("conventions");
    
    if !conventions_dir.exists() {
        anyhow::bail!("conventions/ directory not found");
    }
    
    let mut files = Vec::new();
    
    for entry in fs::read_dir(conventions_dir)
        .context("Failed to read conventions directory")?
    {
        let entry = entry.context("Failed to read directory entry")?;
        let path = entry.path();
        
        if path.is_file() && path.extension().map_or(false, |ext| ext == "md") {
            files.push(ConventionFile::new(path));
        }
    }
    
    // Sort files by name for consistent ordering
    files.sort_by(|a, b| a.name.cmp(&b.name));
    
    Ok(files)
}

fn select_convention_files(files: &[ConventionFile]) -> Result<Vec<ConventionFile>> {
    let file_names: Vec<&str> = files.iter().map(|f| f.name.as_str()).collect();
    
    let selections = MultiSelect::with_theme(&ColorfulTheme::default())
        .with_prompt("Select convention files to symlink")
        .items(&file_names)
        .interact()
        .context("Failed to get file selection")?;
    
    let selected_files: Vec<ConventionFile> = selections
        .into_iter()
        .map(|i| files[i].clone())
        .collect();
    
    Ok(selected_files)
}

fn load_or_create_config() -> Result<Config> {
    let config_path = Path::new("config.yaml");
    
    if config_path.exists() {
        // Load existing config
        let config_content = fs::read_to_string(config_path)
            .context("Failed to read config.yaml")?;
        
        let config: Config = serde_yaml::from_str(&config_content)
            .context("Failed to parse config.yaml")?;
        
        Ok(config)
    } else {
        // Create default config
        let default_config = Config::default();
        let yaml_content = serde_yaml::to_string(&default_config)
            .context("Failed to serialize default config")?;
        
        fs::write(config_path, yaml_content)
            .context("Failed to write config.yaml")?;
        
        println!("{}", style("Created config.yaml with default settings").yellow());
        Ok(default_config)
    }
}

fn find_candidate_directories() -> Result<Vec<PathBuf>> {
    let config = load_or_create_config()
        .context("Failed to load configuration")?;
    
    let mut directories = Vec::new();
    let search_root = PathBuf::from(&config.search_root);
    
    // Validate search root exists
    if !search_root.exists() {
        println!("{}", style(format!("Warning: Search root '{}' doesn't exist, falling back to current directory", config.search_root)).yellow());
        directories.push(PathBuf::from("."));
    } else {
        // Add search root as base
        directories.push(search_root.clone());
        
        // Scan search root for subdirectories
        if let Ok(entries) = fs::read_dir(&search_root) {
            for entry in entries.flatten() {
                if entry.path().is_dir() {
                    directories.push(entry.path());
                }
            }
        }
        
        // Scan one level deeper for nested project directories
        if let Ok(entries) = fs::read_dir(&search_root) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    if let Ok(sub_entries) = fs::read_dir(&path) {
                        for sub_entry in sub_entries.flatten().take(10) { // Increased limit for /Code directory
                            if sub_entry.path().is_dir() {
                                directories.push(sub_entry.path());
                            }
                        }
                    }
                }
            }
        }
    }
    
    // Add current directory as an option
    directories.push(PathBuf::from("."));
    
    // Add parent directory  
    directories.push(PathBuf::from(".."));
    
    // Remove duplicates and sort
    directories.sort();
    directories.dedup();
    
    Ok(directories)
}

fn get_target_directory() -> Result<PathBuf> {
    let candidate_dirs = find_candidate_directories()
        .context("Failed to find candidate directories")?;
    
    let config = load_or_create_config()
        .context("Failed to load configuration")?;
    
    // Create display strings for directories with better formatting
    let dir_items: Vec<String> = candidate_dirs
        .iter()
        .map(|path| {
            let display = path.display().to_string();
            if display == "." {
                "📁 . (current directory)".to_string()
            } else if display == ".." {
                "📁 .. (parent directory)".to_string()
            } else if display.starts_with(&config.search_root) {
                // Show relative path from search root for cleaner display
                let relative = display.strip_prefix(&config.search_root).unwrap_or(&display).trim_start_matches('/');
                format!("📁 ~/{}", relative)
            } else {
                format!("📁 {}", display)
            }
        })
        .collect();
    
    // Add custom path option with distinct icon
    let mut all_items = dir_items.clone();
    all_items.push("✏️  Type custom path...".to_string());
    
    // Visual separator and clear section header
    println!();
    println!("{}", style("╭─────────────────────────────────────────╮").dim());
    println!("{}", style("│          📂 Select Output Directory          │").blue().bold());
    println!("{}", style("╰─────────────────────────────────────────╯").dim());
    println!();
    println!("{}", style("💡 Tip: Start typing to search, press Ctrl+C to cancel").dim().italic());
    println!();
    
    // Setup skim options for clean empty-start UX
    let options = SkimOptionsBuilder::default()
        .height(Some("40%"))
        .multi(false)
        .prompt(Some("🔍 "))
        .header(Some("Press Enter to select, Tab for custom path"))
        .no_hscroll(true)
        .reverse(true)
        .margin(Some("2,4,1,4"))  // Better spacing: top, right, bottom, left
        .min_height(Some("3"))
        .color(Some("16"))  // Use terminal color scheme
        .build()
        .map_err(|e| anyhow::anyhow!("Failed to build skim options: {}", e))?;
    
    // Convert items to skim items
    let item_reader = SkimItemReader::default();
    let items = item_reader.of_bufread(std::io::Cursor::new(all_items.join("\n")));
    
    // Run skim
    let selected_items = Skim::run_with(&options, Some(items))
        .ok_or_else(|| anyhow::anyhow!("No selection made"))?;
    
    // Clear the fuzzy search interface
    print!("\x1B[2J\x1B[1;1H"); // Clear screen
    
    if selected_items.is_abort {
        // Enhanced cancellation flow
        println!();
        println!("{}", style("╭─────────────────────────────────╮").yellow());
        println!("{}", style("│     🚫 Fuzzy Search Cancelled     │").yellow().bold());
        println!("{}", style("╰─────────────────────────────────╯").yellow());
        println!();
        println!("{}", style("💭 No worries! Enter path manually below:").cyan());
        println!();
        
        let custom_input: String = Input::with_theme(&ColorfulTheme::default())
            .with_prompt("📝 Directory path")
            .default(".".to_string())
            .interact_text()
            .context("Failed to get custom directory input")?;
        return Ok(PathBuf::from(custom_input));
    }
    
    let selected = selected_items.selected_items;
    if selected.is_empty() {
        println!("{}", style("📁 Using current directory as default").dim());
        return Ok(PathBuf::from("."));
    }
    
    let selected_text = selected[0].output().to_string();
    
    let target_path = if selected_text.starts_with("✏️") {
        // User selected custom path option
        println!();
        println!("{}", style("╭──────────────────────────────╮").green());
        println!("{}", style("│      ✏️  Custom Path Mode      │").green().bold());
        println!("{}", style("╰──────────────────────────────╯").green());
        println!();
        
        let custom_input: String = Input::with_theme(&ColorfulTheme::default())
            .with_prompt("📝 Enter directory path")
            .default(".".to_string())
            .interact_text()
            .context("Failed to get custom directory input")?;
        PathBuf::from(custom_input)
    } else {
        // Convert display string back to actual path
        let clean_text = selected_text.trim_start_matches("📁 ");
        if clean_text == ". (current directory)" {
            PathBuf::from(".")
        } else if clean_text == ".. (parent directory)" {
            PathBuf::from("..")
        } else if clean_text.starts_with("~/") {
            // Convert relative path back to full path
            PathBuf::from(format!("{}/{}", config.search_root, clean_text.strip_prefix("~/").unwrap_or(clean_text)))
        } else {
            PathBuf::from(clean_text)
        }
    };
    
    // Enhanced directory creation feedback
    if !target_path.exists() {
        println!();
        println!("{}", style("╭─────────────────────────────────╮").yellow());
        println!("{}", style("│      🏗️  Creating Directory      │").yellow().bold());
        println!("{}", style("╰─────────────────────────────────╯").yellow());
        println!();
        println!("{}", style(format!("📁 Creating: {}", target_path.display())).yellow());
        
        fs::create_dir_all(&target_path)
            .context("Failed to create target directory")?;
            
        println!("{}", style("✅ Directory created successfully!").green());
    } else {
        println!();
        println!("{}", style(format!("📁 Selected: {}", target_path.display())).blue().bold());
    }
    
    println!();
    Ok(target_path)
}

fn show_summary(selected_files: &[ConventionFile], target_dir: &Path) -> Result<()> {
    println!();
    println!("{}", style("╭─────────────────────────────────────────╮").cyan());
    println!("{}", style("│             📋 Symlink Summary            │").cyan().bold());
    println!("{}", style("╰─────────────────────────────────────────╯").cyan());
    println!();
    
    println!("{}", style("📄 Selected Convention Files:").green().bold());
    for (i, file) in selected_files.iter().enumerate() {
        println!("   {}. {}", 
                style(format!("{}", i + 1)).dim(), 
                style(&file.name).cyan().bold());
    }
    
    // Show combined filename
    let file_names: Vec<String> = selected_files.iter()
        .map(|f| f.name.replace(".md", ""))
        .collect();
    let combined_filename = format!("{}.md", file_names.join("_"));
    
    println!();
    println!("{}", style("📦 Combined File:").blue().bold());
    println!("   {}", style(format!("combined_conventions/{}", combined_filename)).blue());
    
    println!();
    println!("{}", style("🔗 Symlink Destinations:").blue().bold());
    let symlink_names = ["CONVENTIONS.md", "AGENTS.md", "CLAUDE.md"];
    for name in &symlink_names {
        let target_path = target_dir.join(name);
        println!("   {}", style(target_path.display()).blue().underlined());
    }
    
    // Show agents folder symlink destination
    let agents_target = target_dir.join("AGENTS");
    println!("   {} (folder)", style(agents_target.display()).blue().underlined());
    
    println!();
    println!("{}", style("─".repeat(50)).dim());
    
    Ok(())
}

fn confirm_proceed() -> Result<bool> {
    let proceed = Confirm::with_theme(&ColorfulTheme::default())
        .with_prompt("Proceed with symlink creation?")
        .default(true)
        .interact()
        .context("Failed to get confirmation")?;
    
    Ok(proceed)
}

fn combine_convention_files(selected_files: &[ConventionFile]) -> Result<(String, String)> {
    let mut combined_content = String::new();
    let mut file_names = Vec::new();
    
    for file in selected_files {
        file_names.push(file.name.replace(".md", ""));
        
        let content = fs::read_to_string(&file.path)
            .with_context(|| format!("Failed to read {}", file.name))?;
        
        combined_content.push_str(&content);
        
        // Add newlines between files only if the content doesn't already end with newlines
        if !content.ends_with("\n\n") {
            if content.ends_with('\n') {
                combined_content.push('\n');
            } else {
                combined_content.push_str("\n\n");
            }
        }
    }
    
    // Remove any trailing whitespace
    combined_content = combined_content.trim_end().to_string();
    
    let combined_filename = format!("{}.md", file_names.join("_"));
    
    Ok((combined_content, combined_filename))
}

fn compile_files(selected_files: &[ConventionFile], target_dir: &Path) -> Result<()> {
    println!("{}", style("Combining convention files...").yellow());
    
    // Combine all selected files
    let (combined_content, combined_filename) = combine_convention_files(selected_files)?;
    
    // Create combined_conventions directory
    let combined_dir = Path::new("combined_conventions");
    fs::create_dir_all(combined_dir)
        .context("Failed to create combined_conventions directory")?;
    
    // Write combined file
    let combined_file_path = combined_dir.join(&combined_filename);
    fs::write(&combined_file_path, combined_content)
        .with_context(|| format!("Failed to write combined file {}", combined_filename))?;
    
    println!("  ✅ Created combined file: {}", style(&combined_filename).cyan());
    
    // Create symlinks with the three required names
    let symlink_names = ["CONVENTIONS.md", "AGENTS.md", "CLAUDE.md"];
    
    println!("{}", style("Creating symlinks...").yellow());
    
    for name in &symlink_names {
        let target_path = target_dir.join(name);
        
        // Remove existing symlink/file if it exists
        if target_path.exists() || target_path.is_symlink() {
            fs::remove_file(&target_path)
                .with_context(|| format!("Failed to remove existing {}", name))?;
        }
        
        // Get absolute path for the combined file
        let absolute_combined_path = combined_file_path.canonicalize()
            .with_context(|| format!("Failed to get absolute path for {}", combined_file_path.display()))?;
        
        // Create symlink
        unix_fs::symlink(&absolute_combined_path, &target_path)
            .with_context(|| format!("Failed to create symlink for {}", name))?;
        
        println!("  ✅ Created symlink: {} -> {}", 
                style(name).cyan(), 
                style(absolute_combined_path.display()).dim());
    }
    
    // Create symlink for agents folder
    let agents_source = Path::new("conventions/agents");
    let agents_target = target_dir.join("AGENTS");
    
    if agents_source.exists() {
        // Remove existing symlink/directory if it exists
        if agents_target.exists() || agents_target.is_symlink() {
            if agents_target.is_dir() && !agents_target.is_symlink() {
                fs::remove_dir_all(&agents_target)
                    .context("Failed to remove existing AGENTS directory")?;
            } else {
                fs::remove_file(&agents_target)
                    .context("Failed to remove existing AGENTS symlink")?;
            }
        }
        
        // Get absolute path for the agents folder
        let absolute_agents_path = agents_source.canonicalize()
            .context("Failed to get absolute path for conventions/agents")?;
        
        // Create symlink to agents folder
        unix_fs::symlink(&absolute_agents_path, &agents_target)
            .context("Failed to create symlink for AGENTS folder")?;
        
        println!("  ✅ Created symlink: AGENTS -> {}", 
                style(absolute_agents_path.display()).dim());
    } else {
        println!("  ⚠️  Skipped AGENTS symlink: conventions/agents folder not found");
    }
    
    println!();
    println!("{}", style("✨ All symlinks created successfully!").green().bold());
    
    Ok(())
}
