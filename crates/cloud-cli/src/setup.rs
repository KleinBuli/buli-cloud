use std::io::{Error, Write};

use cloud_core::config::cloud_config::Config;
use dialoguer::{Confirm, Input, Select, theme::ColorfulTheme};
use regex::Regex;

pub fn run_setup() -> Result<Config, Error> {
    let theme = ColorfulTheme::default();
    let version_regex = Regex::new(r"^\d+\.\d+(\.\d+)?$").unwrap();

    loop {
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

        println!("[1/3] Server software\n");

        let server_software_items = ["Paper", "Vanilla"];

        let server_selection = Select::with_theme(&theme)
            .with_prompt("Choose your server software")
            .items(&server_software_items)
            .default(0)
            .interact()?;

        let server_software = server_software_items[server_selection];

        println!("\n[2/3] Proxy software\n");

        let proxy_software_items = ["Velocity"];

        let proxy_selection = Select::with_theme(&theme)
            .with_prompt("Choose your proxy software")
            .items(&proxy_software_items)
            .default(0)
            .interact()?;

        let proxy_software = proxy_software_items[proxy_selection];

        println!("\n[3/3] Minecraft version\n");

        let version: String = Input::with_theme(&theme)
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

        let confirmed = Confirm::with_theme(&theme)
            .with_prompt("Create BuliCloud with this configuration?")
            .default(true)
            .interact()?;

        if !confirmed {
            println!("\nRestarting setup...");
            continue;
        }

        let mut config = Config::default();

        config.set_server_software(server_software);
        config.set_proxy_software(proxy_software);
        config.set_fallback_minecraft_version(&version);

        println!(
            r#"

 ✓ Configuration completed successfully.
 ✓ BuliCloud is ready to initialize.

──────────────────────────────────────────────────
"#
        );

        return Ok(config);
    }
}

fn clear_terminal() {
    print!("\x1B[2J\x1B[1;1H");
    let _ = std::io::stdout().flush();
}
