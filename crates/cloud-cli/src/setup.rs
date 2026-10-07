use std::io::{Error, Write};

use cloud_core::config::cloud_config::Config;
use dialoguer::{Confirm, Input, Select, theme::ColorfulTheme};
use regex::Regex;

pub fn run_setup() -> Result<Config, Error> {
    let theme = ColorfulTheme::default();
    let version_regex = Regex::new(r"^\d+\.\d+(\.\d+)?$").unwrap();

    loop {
        print_welcome();

        let server_software = select_server_software(&theme)?;

        let proxy_software = select_proxy_software(&theme)?;

        let version = read_minecraft_version(&theme, &version_regex)?;

        let confirmed = confirm_configuration(&theme, server_software, proxy_software, &version)?;

        if !confirmed {
            println!("\nRestarting setup...");
            continue;
        }

        let mut config = Config::default();

        config.set_server_software(server_software);
        config.set_proxy_software(proxy_software);
        config.set_fallback_minecraft_version(&version);

        print_completion();

        return Ok(config);
    }
}

fn clear_terminal() {
    print!("\x1B[2J\x1B[1;1H");
    let _ = std::io::stdout().flush();
}

fn print_welcome() {
    clear_terminal();

    println!(
        r#"
  ____        _ _  ____ _                 _
 | __ ) _   _| (_)/ ___| | ___  _   _  __| |
 |  _ \| | | | | | |   | |/ _ \| | | |/ _` |
 | |_) | |_| | | | |___| | (_) | |_| | (_| |
 |____/ \__,_|_|_|\____|_|\___/ \__,_|\__,_|

                Setup Wizard

 Welcome to BuliCloud!
 Let's configure your cloud environment.

──────────────────────────────────────────────────
"#
    );
}

fn select_server_software(theme: &ColorfulTheme) -> Result<&'static str, Error> {
    println!("[1/3] Server software\n");

    let server_software_items = ["Paper", "Vanilla"];

    let server_selection = Select::with_theme(theme)
        .with_prompt("Choose your server software")
        .items(&server_software_items)
        .default(0)
        .interact()?;

    let server_software = server_software_items[server_selection];
    Ok(server_software)
}

fn select_proxy_software(theme: &ColorfulTheme) -> Result<&'static str, Error> {
    println!("\n[2/3] Proxy software\n");

    let proxy_software_items = ["Velocity"];

    let proxy_selection = Select::with_theme(theme)
        .with_prompt("Choose your proxy software")
        .items(&proxy_software_items)
        .default(0)
        .interact()?;

    let proxy_software = proxy_software_items[proxy_selection];
    Ok(proxy_software)
}

fn read_minecraft_version(theme: &ColorfulTheme, version_regex: &Regex) -> Result<String, Error> {
    println!("\n[3/3] Minecraft version\n");

    let version: String = Input::with_theme(theme)
        .with_prompt("Fallback Minecraft version")
        .default("1.21.11".to_string())
        .validate_with(|input: &String| -> Result<(), &str> {
            if version_regex.is_match(input) {
                Ok(())
            } else {
                Err("Use a version like 1.21 or 1.21.11")
            }
        })
        .interact_text()?;
    Ok(version)
}

fn confirm_configuration(theme: &ColorfulTheme, server_software: &str, proxy_software: &str, version: &str) -> Result<bool, Error> {
    println!(
        r#"

──────────────────────────────────────────────────

 Configuration Summary

   Server software      : {server_software}
   Proxy software       : {proxy_software}
   Minecraft version    : {version}

──────────────────────────────────────────────────
"#
    );

    let confirmed = Confirm::with_theme(theme)
        .with_prompt("Create BuliCloud with this configuration?")
        .default(true)
        .interact()?;
    Ok(confirmed)
}

fn print_completion() {
    println!(
        r#"

 ✓ Configuration completed successfully.
 ✓ BuliCloud is ready to initialize.

──────────────────────────────────────────────────
"#
    );
}
