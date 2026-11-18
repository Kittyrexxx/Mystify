use colored::*;
use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    terminal::{disable_raw_mode, enable_raw_mode},
};
use base64::{Engine, engine::general_purpose};
use std::process::Command;
use std::io::{self, Write};
use std::fs;
use std::fs::File;
use std::io::{Seek, SeekFrom};
use rand::Rng;
use regex::Regex;
use std::thread;
use std::time::Duration;
use std::env;
use std::path::Path;

const CURRENT_VERSION: &str = "1.2.1";
const GITHUB_RELEASES_API: &str = "https://api.github.com/repos/kittyrexxx/mystify/releases/latest";

#[cfg(windows)]
fn enable_ansi_support() {
    use winapi::um::consoleapi::{GetConsoleMode, SetConsoleMode};
    use winapi::um::processenv::GetStdHandle;
    use winapi::um::winbase::STD_OUTPUT_HANDLE;
    unsafe {
        let handle = GetStdHandle(STD_OUTPUT_HANDLE);
        let mut mode = 0;
        if GetConsoleMode(handle, &mut mode) != 0 {
            SetConsoleMode(handle, mode | 0x0004);
        }
    }
}

#[cfg(not(windows))]
fn enable_ansi_support() {}

fn clear_screen() {
    print!("\x1b[H");
    let _ = io::stdout().flush();
}

fn hard_clear() {
    print!("\x1b[2J\x1b[1;1H");
    let _ = io::stdout().flush();
}

fn print_header(current_desc: &str) {
    clear_screen();
    println!("{}", "   __  __           _   _  __       ".cyan().bold());
    println!("{}", "  |  \\/  |_   _ ___| |_(_)/ _|_   _ ".cyan().bold());
    println!("{}", "  | |\\/| | | | / __| __| |  _| | | |".cyan().bold());
    println!("{}", "  | |  | | |_| \\__ \\ |_| | | | |_| |".cyan().bold());
    println!("{}", "  |_|  |_|\\__, |___/\\__|_|_|  \\__, |".cyan().bold());
    println!("{}", "           |___/               |___/ ".cyan().bold());
    println!("{}", format!("        v{} - OPEN SOURCE", CURRENT_VERSION).green().bold());
    println!();
    println!("{}", format!("> {}", current_desc).yellow().bold());
    println!();
}

fn select_with_desc(prompt_title: &str, options: Vec<&str>, descriptions: Vec<&str>) -> Option<usize> {
    enable_raw_mode().unwrap();
    let mut selected = 0;
    let mut search_query = String::new();

    hard_clear();

    loop {
        let query_lower = search_query.to_lowercase();
        let filtered_indices: Vec<usize> = options
            .iter()
            .enumerate()
            .filter(|(_, opt)| query_lower.is_empty() || opt.to_lowercase().contains(&query_lower))
            .map(|(i, _)| i)
            .collect();

        if selected >= filtered_indices.len() && !filtered_indices.is_empty() {
            selected = filtered_indices.len() - 1;
        } else if filtered_indices.is_empty() {
            selected = 0;
        }

        let current_desc = if !filtered_indices.is_empty() && filtered_indices[selected] < descriptions.len() {
            descriptions[filtered_indices[selected]]
        } else {
            "Select an option."
        };

        // Snap cursor to top-left for flicker-free rendering
        print!("\x1b[H");
        
        println!("\x1b[K{}", "   __  __           _   _  __       ".cyan().bold());
        println!("\x1b[K{}", "  |  \\/  |_   _ ___| |_(_)/ _|_   _ ".cyan().bold());
        println!("\x1b[K{}", "  | |\\/| | | | / __| __| |  _| | | |".cyan().bold());
        println!("\x1b[K{}", "  | |  | | |_| \\__ \\ |_| | | | |_| |".cyan().bold());
        println!("\x1b[K{}", "  |_|  |_|\\__, |___/\\__|_|_|  \\__, |".cyan().bold());
        println!("\x1b[K{}", "           |___/               |___/ ".cyan().bold());
        println!("\x1b[K{}", format!("        v{} - OPEN SOURCE", CURRENT_VERSION).green().bold());
        println!("\x1b[K");
        println!("\x1b[K{}", format!("> {}", current_desc).yellow().bold());
        println!("\x1b[K");

        let filter_display = if search_query.is_empty() { "".to_string() } else { format!("[Filter: {}]", search_query) };
        println!("\x1b[K{} {}", prompt_title.cyan().bold(), filter_display.yellow());
        println!("\x1b[K{}", "Type to filter, ↑↓ to move, Enter to select, Backspace to clear".truecolor(128, 128, 128));
        println!("\x1b[K");

        if filtered_indices.is_empty() {
            println!("\x1b[K  {}", "No matches found.".red());
        } else {
            for (display_idx, &orig_idx) in filtered_indices.iter().enumerate() {
                let opt = options[orig_idx];
                if display_idx == selected {
                    println!("\x1b[K{} {}", ">".green().bold(), opt.green().bold());
                } else {
                    println!("\x1b[K  {}", opt.white());
                }
            }
        }
        
        // Clear any leftover lines below the menu frame
        for _ in 0..5 {
            println!("\x1b[K");
        }
        let _ = io::stdout().flush();

        if let Ok(Event::Key(key_event)) = event::read() {
            if key_event.kind == KeyEventKind::Press {
                match key_event.code {
                    KeyCode::Up => {
                        if !filtered_indices.is_empty() {
                            if selected > 0 {
                                selected -= 1;
                            } else {
                                selected = filtered_indices.len() - 1;
                            }
                        }
                    }
                    KeyCode::Down => {
                        if !filtered_indices.is_empty() {
                            if selected < filtered_indices.len() - 1 {
                                selected += 1;
                            } else {
                                selected = 0;
                            }
                        }
                    }
                    KeyCode::Backspace => {
                        search_query.pop();
                        selected = 0;
                    }
                    KeyCode::Char(c) => {
                        if !c.is_control() {
                            search_query.push(c);
                        }
                        selected = 0;
                    }
                    KeyCode::Enter => {
                        disable_raw_mode().unwrap();
                        hard_clear();
                        if !filtered_indices.is_empty() {
                            return Some(filtered_indices[selected]);
                        }
                    }
                    KeyCode::Esc => {
                        disable_raw_mode().unwrap();
                        hard_clear();
                        return None;
                    }
                    _ => {}
                }
            }
        }
    }
} 

fn pick_file() -> Option<String> {
    let file = rfd::FileDialog::new().pick_file();
    file.map(|p| p.to_string_lossy().to_string())
}

fn pick_folder() -> Option<String> {
    let folder = rfd::FileDialog::new().pick_folder();
    folder.map(|p| p.to_string_lossy().to_string())
}

fn get_multiline_input(prompt_text: &str) -> String {
    hard_clear();
    print_header(prompt_text);
    println!("{} Opening text input window...", "info".yellow().bold());
    let ps_script = format!(
        r#"
        Add-Type -AssemblyName System.Windows.Forms;
        Add-Type -AssemblyName System.Drawing;
        $form = New-Object System.Windows.Forms.Form;
        $form.Text = '{}';
        $form.Size = New-Object System.Drawing.Size(600, 400);
        $form.StartPosition = 'CenterScreen';
        $form.TopMost = $true;
        $textBox = New-Object System.Windows.Forms.TextBox;
        $textBox.Multiline = $true;
        $textBox.ScrollBars = 'Vertical';
        $textBox.Dock = 'Fill';
        $textBox.Font = New-Object System.Drawing.Font('Consolas', 11);
        $form.Controls.Add($textBox);
        $button = New-Object System.Windows.Forms.Button;
        $button.Text = 'Submit';
        $button.Dock = 'Bottom';
        $button.Add_Click({{ $form.Close() }});
        $form.Controls.Add($button);
        $form.ShowDialog() | Out-Null;
        Write-Output $textBox.Text;
        "#, prompt_text
    );
    let output = Command::new("powershell").arg("-NoProfile").arg("-Command").arg(&ps_script).output().expect("Failed to open textbox");
    String::from_utf8_lossy(&output.stdout).trim().to_string()
}

fn show_output(output: &str, title: &str, heavy: bool) {
    hard_clear();
    if heavy {
        let mut child = Command::new("clip").stdin(std::process::Stdio::piped()).spawn().unwrap();
        if let Some(mut stdin) = child.stdin.take() { stdin.write_all(output.as_bytes()).unwrap(); }
        child.wait().unwrap();
        let temp_dir = std::env::temp_dir();
        let file_path = temp_dir.join(format!("mystify_{}.txt", title));
        fs::write(&file_path, output).unwrap();
        Command::new("notepad").arg(&file_path).spawn().unwrap();
        println!("\n{} Copied to clipboard & opened in Notepad!", "success".green().bold());
    } else {
        println!("\n{} Output:\n{}", "success".green().bold(), output.yellow().bold());
    }
}

fn get_input(prompt_text: &str, header_desc: &str) -> String {
    hard_clear();
    print_header(header_desc);
    inquire::Text::new(prompt_text).prompt().unwrap_or_default()
}

fn check_for_updates() {
    hard_clear();
    print_header("Checks GitHub releases for the latest version and downloads updates automatically.");
    print!("{} Checking GitHub for updates...", "net".cyan().bold());
    let _ = io::stdout().flush();
    thread::sleep(Duration::from_millis(1000));

    let client = reqwest::blocking::Client::builder()
        .user_agent("mystify-updater")
        .build();

    if let Ok(c) = client {
        let res = c.get(GITHUB_RELEASES_API).send();
        if let Ok(response) = res {
            if let Ok(json) = response.json::<serde_json::Value>() {
                if let Some(tag_name) = json["tag_name"].as_str() {
                    let latest_clean = tag_name.trim_start_matches('v');
                    if latest_clean == CURRENT_VERSION {
                        hard_clear();
                        println!("{} You are running the latest version (v{}).", "success".green().bold(), CURRENT_VERSION);
                        pause();
                        return;
                    } else {
                        hard_clear();
                        println!("{} A new update is available! (v{} -> {})", "info".yellow().bold(), CURRENT_VERSION, tag_name);
                        
                        let confirm = vec!["Yes, Download & Update", "Cancel"];
                        let confirm_desc = vec!["Download and install the latest release.", "Return to main menu without updating."];
                        if let Some(choice_idx) = select_with_desc("Do you want to download the update?", confirm, confirm_desc) {
                            if choice_idx == 0 {
                                if let Some(assets) = json["assets"].as_array() {
                                    if let Some(asset) = assets.first() {
                                        if let Some(download_url) = asset["browser_download_url"].as_str() {
                                            hard_clear();
                                            println!("{} Downloading update from GitHub...", "success".green().bold());
                                            
                                            if let Ok(mut resp) = c.get(download_url).send() {
                                                let new_file_name = format!("mystify_{}.exe", tag_name);
                                                let mut out = fs::File::create(&new_file_name).unwrap();
                                                io::copy(&mut resp, &mut out).unwrap();
                                                
                                                hard_clear();
                                                println!("{} Update downloaded successfully!", "success".green().bold());
                                                println!("{} Applying update and restarting...", "info".yellow().bold());

                                                let current_exe = env::current_exe().expect("Failed to get current exe");
                                                let current_exe_str = current_exe.to_str().unwrap();
                                                
                                                let batch_script = format!(
                                                    "@echo off\n\
                                                    timeout /t 2 /nobreak > NUL\n\
                                                    del /f /q \"{}\"\n\
                                                    ren \"{}\" \"mystify.exe\"\n\
                                                    start \"\" \"mystify.exe\"\n\
                                                    del \"%~f0\"\n",
                                                    current_exe_str, new_file_name
                                                );

                                                fs::write("update.bat", batch_script).expect("Failed to write update script");
                                                Command::new("cmd.exe").args(&["/C", "update.bat"]).spawn().expect("Failed to launch updater");
                                                std::process::exit(0);
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        pause();
                        return;
                    }
                }
            }
        }
    }
    hard_clear();
    println!("{} Could not check for updates from GitHub.", "error".red().bold());
    pause();
}

fn main() {
    enable_ansi_support();
    hard_clear();

    if Path::new("update.bat").exists() {
        let _ = fs::remove_file("update.bat");
    }

    let options = vec![
        "1. Check for Updates",
        "2. Code Obfuscator",
        "3. Encoding & Decoding",
        "4. Network & Web Tools",
        "5. Extractors",
        "6. Privacy",
        "7. Restart Mystify",
        "8. Exit",
    ];

    let descriptions = vec![
        "Checks GitHub releases for newer versions and updates Mystify automatically.",
        "Protects your scripts with encryption.",
        "Encode & Decode in various formats.",
        "Perform network diagnostics and web utility tasks.",
        "Extract certain contents from text (Emails, Phone numbers, ETC).",
        "Securely overwrite and delete files or folders.",
        "Restart mystify.",
        "Exit mystify.",
    ];

    loop {
        if let Some(idx) = select_with_desc("Select category (Type to search):", options.clone(), descriptions.clone()) {
            match idx {
                0 => check_for_updates(),
                1 => {
                    hard_clear();
                    print_header("Protects your scripts with encryption.");
                    println!("{}", "The Code Obfuscator is currently being rewritten.".yellow().bold());
                    pause();
                },
                2 => encoding_menu(),
                3 => network_menu(),
                4 => extract_menu(),
                5 => privacy_menu(),
                6 => restart_app(),
                7 => std::process::exit(0),
                _ => {}
            }
        } else {
            break;
        }
    }
}

fn encoding_menu() {
    loop {
        let action_opts = vec!["Encode", "Decode", "Back"];
        let action_desc = vec!["Convert text into an encoded format.", "Convert encoded text back to readable text.", "Return to main menu."];
        
        match select_with_desc("Encoding - Select action:", action_opts.clone(), action_desc) {
            Some(2) | None => break,
            Some(action_idx) => {
                let is_encode = action_idx == 0;
                
                let formats = if is_encode {
                    vec!["Base64", "Hexadecimal", "Binary", "Octal", "URL Encoding", "ASCII/Decimal", "ROT Shift (1-25)", "Morse Code", "Atbash", "A1Z26", "Reverse String", "Leetspeak", "Back"]
                } else {
                    vec!["SMART", "Base64", "Hexadecimal", "Binary", "Octal", "URL Encoding", "ASCII/Decimal", "ROT Shift (1-25)", "Morse Code", "Atbash", "A1Z26", "Reverse String", "Leetspeak", "Back"]
                };
                
                let format_desc = if is_encode {
                    vec![
                        "Standard Base64 encoding.", "Base-16 encoding.", 
                        "Binary representation.", "Octal representation.", "URL percent-encoding.", 
                        "Numeric ASCII values.", "All 25 rotation shifts at once.", "Dots and dashes.", 
                        "Reversed alphabet cipher.", "Numbers 1-26 for letters.", "Reverse the text.", 
                        "Replace letters with numbers/symbols.", "Return to previous menu."
                    ]
                } else {
                    vec![
                        "Auto-detects the format and decodes it.", "Standard Base64 encoding.", "Base-16 encoding.", 
                        "Binary representation.", "Octal representation.", "URL percent-encoding.", 
                        "Numeric ASCII values.", "All 25 rotation shifts at once.", "Dots and dashes.", 
                        "Reversed alphabet cipher.", "Numbers 1-26 for letters.", "Reverse the text.", 
                        "Replace letters with numbers/symbols.", "Return to previous menu."
                    ]
                };

                if let Some(fmt_idx) = select_with_desc(&format!("{} - Select format:", action_opts[action_idx]), formats.clone(), format_desc) {
                    if formats[fmt_idx] == "Back" { continue; }
                    
                    let text = get_multiline_input(&format!("Enter text to {}:", action_opts[action_idx].to_lowercase()));
                    if text.is_empty() { continue; }
                    
                    let result = if is_encode {
                        match formats[fmt_idx] {
                            "Base64" => general_purpose::STANDARD.encode(text.as_bytes()),
                            "Hexadecimal" => hex::encode(text.as_bytes()),
                            "Binary" => text.bytes().map(|b| format!("{:08b}", b)).collect::<Vec<_>>().join(" "),
                            "Octal" => text.bytes().map(|b| format!("{:03o}", b)).collect::<Vec<_>>().join(" "),
                            "URL Encoding" => urlencoding::encode(&text).to_string(),
                            "ASCII/Decimal" => text.bytes().map(|b| b.to_string()).collect::<Vec<_>>().join(" "),
                            "ROT Shift (1-25)" => {
                                let mut out = String::new();
                                for shift in 1..=25 {
                                    let shifted: String = text.chars().map(|c| rot_shift(c, shift)).collect();
                                    out.push_str(&format!("ROT{}: {}\n", shift, shifted));
                                }
                                out
                            },
                            "Morse Code" => to_morse(&text),
                            "Atbash" => atbash(&text),
                            "A1Z26" => a1z26_encode(&text),
                            "Reverse String" => text.chars().rev().collect(),
                            "Leetspeak" => leetspeak(&text),
                            _ => "Error".to_string()
                        }
                    } else {
                        match formats[fmt_idx] {
                            "SMART" => smart_decode(&text),
                            "Base64" => general_purpose::STANDARD.decode(text.as_bytes()).map(|v| String::from_utf8_lossy(&v).to_string()).unwrap_or_else(|_| "Error".to_string()),
                            "Hexadecimal" => hex::decode(text.as_bytes()).map(|v| String::from_utf8_lossy(&v).to_string()).unwrap_or_else(|_| "Error".to_string()),
                            "ROT Shift (1-25)" => {
                                let mut out = String::new();
                                for shift in 1..=25 {
                                    let shifted: String = text.chars().map(|c| rot_shift(c, shift)).collect();
                                    out.push_str(&format!("ROT{}: {}\n", shift, shifted));
                                }
                                out
                            },
                            "Atbash" => atbash(&text),
                            "A1Z26" => a1z26_decode(&text),
                            "Reverse String" => text.chars().rev().collect(),
                            _ => "Decode not applicable for this format.".to_string()
                        }
                    };
                    show_output(&result, "Encoded", true);
                    pause();
                }
            }
        }
    }
}

fn privacy_menu() {
    let opts = vec!["1. File Shredder (3 passes)", "Back"];
    let descs = vec![
        "Securely overwrite and delete files or folders.",
        "Return to main menu."
    ];

    loop {
        if let Some(idx) = select_with_desc("Privacy - Select action:", opts.clone(), descs.clone()) {
            if idx == 1 || opts[idx] == "Back" {
                break;
            } else if idx == 0 {
                let mode_opts = vec!["File", "Folder", "Back"];
                let mode_descs = vec!["Shred a single file.", "Shred every file in a directory.", "Return to privacy menu."];
                if let Some(midx) = select_with_desc("Select target type:", mode_opts.clone(), mode_descs.clone()) {
                    if mode_opts[midx] == "Back" { continue; }
                    let path_str = if mode_opts[midx] == "Folder" { pick_folder() } else { pick_file() };
                    if let Some(p) = path_str {
                        let path = Path::new(&p);
                        if path.is_dir() { shred_dir(path); } else { shred_file(path); }
                        hard_clear();
                        println!("\n{} Shredding complete!", "success".green().bold());
                        pause();
                    }
                }
            }
        } else {
            break;
        }
    }
}

fn shred_file(path: &Path) {
    let metadata = match fs::metadata(path) { Ok(m) => m, Err(_) => return };
    let size = metadata.len();
    if size == 0 { let _ = fs::remove_file(path); return; }

    for pass in 1..=3 {
        hard_clear();
        print!("{} Pass {}/3: ", "shred".cyan().bold(), pass);
        let _ = io::stdout().flush();
        
        if let Ok(mut file) = File::options().write(true).open(path) {
            let _ = file.seek(SeekFrom::Start(0));
            let mut written = 0;
            let mut rng = rand::thread_rng();
            let mut buffer = [0u8; 4096];

            while written < size {
                let to_write = std::cmp::min(4096, (size - written) as usize);
                if pass == 1 { rng.fill(&mut buffer); } 
                else if pass == 2 { buffer = [0u8; 4096]; } 
                else { buffer = [1u8; 4096]; }

                let _ = file.write_all(&buffer[..to_write]);
                written += to_write as u64;

                let percent = (written as f64 / size as f64 * 100.0) as u8;
                print!("\r{} Pass {}/3: [{}%] ", "shred".cyan().bold(), pass, percent);
                let _ = io::stdout().flush();
            }
        }
        println!(); 
    }
    let _ = fs::remove_file(path);
    hard_clear();
    println!("{} File deleted.", "success".green().bold());
}

fn shred_dir(path: &Path) {
    if let Ok(entries) = fs::read_dir(path) {
        for entry in entries.flatten() {
            let p = entry.path();
            if p.is_dir() {
                shred_dir(&p);
                let _ = fs::remove_dir(&p);
            } else {
                shred_file(&p);
            }
        }
    }
}

fn extract_menu() {
    let opts = vec![
        "Extract Emails", "Extract URLs", "Extract IP Addresses", 
        "Extract Phone Numbers", "Extract Hex Colors", "Extract Discord Invites", "Back"
    ];
    let descs = vec![
        "Extract email addresses from text or files.",
        "Extract web URLs from text or files.",
        "Extract IPv4 addresses from text or files.",
        "Extract phone numbers from text or files.",
        "Extract hex color codes from text or files.",
        "Extract Discord invite links from text or files.",
        "Return to main menu."
    ];

    loop {
        if let Some(idx) = select_with_desc("Extractors - Select action:", opts.clone(), descs.clone()) {
            if opts[idx] == "Back" { break; }
            
            let input_method = vec!["Paste Text", "Read File", "Back"];
            let method_descs = vec!["Paste raw text directly via a popup window.", "Select a file from your disk to read.", "Return to extractors menu."];

            if let Some(midx) = select_with_desc("Input method:", input_method.clone(), method_descs.clone()) {
                if input_method[midx] == "Back" { continue; }
                let text = if input_method[midx] == "Read File" {
                    if let Some(path) = pick_file() { fs::read_to_string(path).unwrap_or_default() } else { continue; }
                } else {
                    get_multiline_input("Paste text to extract from:")
                };
                
                if text.is_empty() { continue; }
                let c = opts[idx];
                let regex_str = match c {
                    "Extract Emails" => r"[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}",
                    "Extract URLs" => r"https?://[^\s/$.?#].[^\s]*",
                    "Extract IP Addresses" => r"\b(?:\d{1,3}\.){3}\d{1,3}\b",
                    "Extract Phone Numbers" => r"\+?\d{1,4}?[-.\s]?\(?\d{1,3}?\)?[-.\s]?\d{1,4}[-.\s]?\d{1,4}[-.\s]?\d{1,9}",
                    "Extract Hex Colors" => r"#(?:[0-9a-fA-F]{3}){1,2}\b",
                    "Extract Discord Invites" => r"(?:discord\.gg/|discord\.com/invite/)[a-zA-Z0-9]+",
                    _ => ""
                };
                if !regex_str.is_empty() {
                    let re = Regex::new(regex_str).unwrap();
                    let matches: Vec<&str> = re.find_iter(&text).map(|m| m.as_str()).collect();
                    hard_clear();
                    if matches.is_empty() { println!("{} No matches found.", "error".red().bold()); } 
                    else { show_output(&matches.join("\n"), "Extracted", true); }
                    pause();
                }
            }
        } else {
            break;
        }
    }
}

fn network_menu() {
    let opts = vec!["1. Ping Host", "2. DNS Lookup", "3. Get Local IP", "4. Flush DNS Cache", "5. Back"];
    let descs = vec![
        "Ping a remote host to test connection.",
        "Resolve a domain name to its IP address.",
        "Display your local machine IP address.",
        "Flush the local DNS resolver cache.",
        "Return to main menu."
    ];

    loop {
        if let Some(idx) = select_with_desc("Network - Select action:", opts.clone(), descs.clone()) {
            if opts[idx] == "5. Back" { break; }
            match opts[idx] {
                "1. Ping Host" => { 
                    let ip = get_input("Enter IP or Domain:", "Ping a remote host or IP address."); 
                    if !ip.is_empty() { 
                        let out = Command::new("ping").arg("-n").arg("1").arg(&ip).output(); 
                        hard_clear();
                        if let Ok(o) = out { show_output(&String::from_utf8_lossy(&o.stdout), "Ping", false); } 
                        pause(); 
                    } 
                }
                "2. DNS Lookup" => { 
                    let host = get_input("Enter domain:", "Perform a DNS lookup for a domain."); 
                    if !host.is_empty() { 
                        use std::net::ToSocketAddrs; 
                        hard_clear();
                        match (host.as_str(), 80).to_socket_addrs() { 
                            Ok(mut a) => { if let Some(ad) = a.next() { show_output(&format!("{} -> {}", host, ad.ip()), "DNS", false); } } 
                            Err(_) => println!("{} Failed.", "error".red().bold()), 
                        } 
                        pause(); 
                    } 
                }
                "3. Get Local IP" => { 
                    hard_clear();
                    match local_ip_address::local_ip() { 
                        Ok(ip) => show_output(&ip.to_string(), "IP", false), 
                        Err(_) => println!("{} Error.", "error".red().bold()), 
                    } 
                    pause(); 
                }
                "4. Flush DNS Cache" => {
                    hard_clear();
                    print!("{} Flushing DNS cache... ", "net".cyan().bold());
                    let _ = io::stdout().flush();
                    let out = Command::new("ipconfig").arg("/flushdns").output();
                    match out {
                        Ok(o) => {
                            println!("{} Done!", "success".green().bold());
                            show_output(&String::from_utf8_lossy(&o.stdout), "FlushDNS", false);
                        }
                        Err(_) => println!("{} Failed.", "error".red().bold()),
                    }
                    pause();
                }
                _ => break,
            }
        } else {
            break;
        }
    }
}

fn rot_shift(c: char, shift: u8) -> char {
    if c.is_ascii_alphabetic() {
        let base = if c.is_ascii_lowercase() { b'a' } else { b'A' };
        (((c as u8 - base + shift) % 26) + base) as char
    } else { c }
}

fn atbash(text: &str) -> String {
    text.chars().map(|c| {
        if c.is_ascii_lowercase() { (b'z' - (c as u8 - b'a')) as char }
        else if c.is_ascii_uppercase() { (b'Z' - (c as u8 - b'A')) as char }
        else { c }
    }).collect()
}

fn a1z26_encode(text: &str) -> String {
    text.to_lowercase().chars().filter(|c| c.is_ascii_alphabetic()).map(|c| (c as u8 - b'a' + 1).to_string()).collect::<Vec<_>>().join("-")
}

fn a1z26_decode(text: &str) -> String {
    text.split('-').filter_map(|s| s.parse::<u8>().ok()).filter_map(|n| if n >= 1 && n <= 26 { Some((b'a' + n - 1) as char) } else { None }).collect()
}

fn leetspeak(text: &str) -> String {
    text.chars().map(|c| match c.to_ascii_lowercase() {
        'a' => '4', 'e' => '3', 'i' => '1', 'o' => '0', 's' => '5', 't' => '7', 'l' => '1',
        _ => c
    }).collect()
}

fn to_morse(text: &str) -> String {
    let morse = vec![
        (".-", "A"), ("-...", "B"), ("-.-.", "C"), ("-..", "D"), (".", "E"), ("..-.", "F"),
        ("--.", "G"), ("....", "H"), ("..", "I"), (".---", "J"), ("-.-", "K"), (".-..", "L"),
        ("--", "M"), ("-.", "N"), ("---", "O"), (".--.", "P"), ("--.-", "Q"), (".-.", "R"),
        ("...", "S"), ("-", "T"), ("..-", "U"), ("...-", "V"), (".--", "W"), ("-..-", "X"),
        ("-.--", "Y"), ("--..", "Z"), ("-----", "0"), (".----", "1"), ("..---", "2"), ("...--", "3"),
        ("....-", "4"), (".....", "5"), ("-....", "6"), ("--...", "7"), ("---..", "8"), ("----.", "9")
    ];
    text.to_uppercase().chars().map(|c| {
        let s = c.to_string();
        morse.iter().find(|&&(_, letter)| letter == s).map(|&(code, _)| code).unwrap_or("/")
    }).collect::<Vec<_>>().join(" ")
}

fn smart_decode(text: &str) -> String {
    if let Ok(d) = general_purpose::STANDARD.decode(text.as_bytes()) { return format!("Base64: {}", String::from_utf8_lossy(&d)); }
    if let Ok(d) = hex::decode(text) { return format!("Hex: {}", String::from_utf8_lossy(&d)); }
    if text.contains('%') { if let Ok(d) = urlencoding::decode(text) { return format!("URL: {}", d); } }
    "Could not auto-detect format.".to_string()
}

fn restart_app() {
    let exe_path = std::env::current_exe().unwrap();
    Command::new(exe_path).spawn().expect("Failed to restart");
    std::process::exit(0);
}

fn pause() {
    print!("\n{} Press Enter to continue...", "info".yellow().bold());
    let _ = io::stdout().flush();
    let mut buffer = String::new();
    let _ = io::stdin().read_line(&mut buffer);
}
