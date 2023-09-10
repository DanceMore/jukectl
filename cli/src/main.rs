extern crate clap;
extern crate colored;
extern crate dotenv;
extern crate log;
extern crate reqwest;
extern crate serde;
extern crate tokio;

use clap::{App, Arg};
use colored::*;
use dotenv::dotenv;
use log::{error, warn, info, debug};
use serde::Deserialize;
use serde::Serialize;
use serde_json;
use std::fmt;
use std::process::exit;

// painstakeingly hand-crafted ASCII art coloring
// because street culture is life
fn print_banner() {
    println!("{}{}{}",    "       __".red().bold(), "       __".green().bold(),"                  __  .__    ".blue().bold());
    println!("{}{}{}{}{}",  "      |__|".red().bold(),"__ __".yellow().bold(), "|  | __".green().bold()," ____ ".cyan().bold(),  "  _____/  |_|  |   ".blue().bold());
    println!("{}{}{}{}{}",  "      |  |".red().bold(),"  |  \\".yellow().bold(),"  |/ /".green().bold(),"/ __ \\".cyan().bold(), "_/ ___\\   __\\  |   ".blue().bold());
    println!("{}{}{}{}{}",  "      |  |".red().bold(),"  |  /".yellow().bold(),"    <".green().bold(), "\\  ___/".cyan().bold(), "\\  \\___|  | |  |__ ".blue().bold());
    println!("{}{}{}{}{}", "  /\\__|  |".red().bold(),"____/".yellow().bold(), "|__|_ \\".green().bold(),"\\___  >".cyan().bold(),"\\___  >__| |____/ ".blue().bold());
    println!("{}{}{}{}{}", "  \\______|".red().bold(),"     ".yellow().bold(), "     \\/ ".green().bold(),"   \\/ ".cyan().bold(),"    \\/            ".blue().bold());
}

// TagData, useful holder for any_tags vs not_tags
#[derive(Serialize, Deserialize)]
struct TagsData {
    any: Vec<String>,
    not: Vec<String>,
}

// Implement the Debug trait for TagsData
impl<> fmt::Debug for TagsData<> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{{\n    any: {:?},\n    not: {:?}\n}}", self.any, self.not)
    }
}

impl<> TagsData<> {
    fn to_json(&self) -> String {
        match serde_json::to_string(self) {
            Ok(json) => json,
            Err(err) => {
                eprintln!("[!!!] Error serializing TagsData to JSON: {}", err);
                std::process::exit(1); // Exit with a non-zero status code
            }
        }
    }
}

fn parse_tags_data(tags: &str, not_tags: &str) -> TagsData {
    TagsData {
        any: tags.split(',').map(|s| s.trim().to_string()).collect(),
        not: not_tags.split(',').map(|s| s.trim().to_string()).collect(),
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize the logger
    env_logger::init();

    // Load environment variables from .env file
    dotenv().ok();

    // make compiler warning quiet; it should be getting set or exiting.
    #[allow(unused_assignments)]
    let mut api_hostname = "http://default-api-hostname.com".to_string();

    // Access the JUKECTL_HOST environment variable
    if let Ok(hostname) = std::env::var("JUKECTL_HOST") {
        api_hostname = hostname;
        info!("[-] jukectl API base URL: {}", api_hostname);
    } else {
        eprintln!("Error: JUKECTL_HOST environment variable is not set.");
        exit(1);
    }

    // clap crate, giving me almost Ruby-Thor library vibes and easy
    // command-line arg parsing :D
    let matches = App::new("jukectl")
        .about("command-line remote control for jukectl music player service")
        .subcommand(
            App::new("status")
                .about("Display current status of service")
        )
        .subcommand(
            App::new("tag")
                .about("Tag an item")
                .arg(
                    Arg::with_name("TagName")
                        .help("Name of the tag")
                        .required(true),
                ),
        )
        .subcommand(
            App::new("untag")
                .about("Untag an item")
                .arg(
                    Arg::with_name("TagName")
                        .help("Name of the tag")
                        .required(true),
                ),
        )
        .subcommand(App::new("skip").about("Skip an item"))
        .subcommand(
            App::new("playback")
                .about("Playback with tags")
                .arg(
                    Arg::with_name("tags")
                        .help("Tags for playback")
                        .required(true),
                )
                .arg(
                    Arg::with_name("not_tags")
                        .help("Tags to exclude from playback")
                        .required(false),
                ),
        )
        .get_matches();


    // Handle subcommands
    match matches.subcommand() {
        ("status", _) => {
            match status(&api_hostname).await {
                Ok(_) => debug!("[-] status() function completed successfully."),
                Err(err) => eprintln!("[!] Error: {}", err),
            }
        }
	("tag", Some(tag_matches)) => {
            let add_tag = tag_matches.value_of("TagName").unwrap().to_string();
            tag(&api_hostname, add_tag).await?;
        }
	("untag", Some(untag_matches)) => {
            let remove_tag = untag_matches.value_of("TagName").unwrap().to_string();
            untag(&api_hostname, remove_tag).await?;
        }
        ("skip", Some(_)) => {
            match skip_item(&api_hostname).await {
                Ok(_) => debug!("[-] skip() function completed successfully."),
                Err(err) => eprintln!("[!] Error: {}", err),
            }
        }
        ("playback", Some(playback_matches)) => {
            let tags = playback_matches.value_of("tags").unwrap_or("");
            let not_tags = playback_matches.value_of("not_tags").unwrap_or("");

            let tags_data = parse_tags_data(tags, not_tags);

            match playback(&api_hostname, &tags_data).await {
                Ok(_) => debug!("[-] playback() function completed successfully."),
                Err(err) => eprintln!("[!] Error: {}", err),
            }
        }
        _ => println!("Invalid subcommand. Use 'jukectl --help' for usage."),
    }

    Ok(())
}

async fn status(api_hostname: &str) -> Result<(), reqwest::Error> {
    print_banner();

    let client = reqwest::Client::new();
    let url = format!("{}/tags", api_hostname);

    let response = client.get(&url).send().await?;

    if response.status().is_success() {
        let body = response.text().await?;
        debug!("[?] raw response body: {}", body);

        // Attempt to deserialize the JSON response into TagsData
        match serde_json::from_str::<TagsData>(&body) {
            Ok(tags_data) => {
                println!("{}", "current playback tags:".cyan().bold());
                println!("    {}: {:?}", "any".green().bold(), tags_data.any);
                println!("    {}: {:?}", "not".red().bold(), tags_data.not);
            }
            Err(e) => {
                eprintln!("Error: Failed to deserialize response: {}", e);
            }
        }

        // Make an additional GET request to the root URL
        let root_url = format!("{}/", api_hostname);
        let root_response = client.get(&root_url).send().await?;

        if root_response.status().is_success() {
            let root_body = root_response.text().await?;
            debug!("[?] raw root response body: {}", root_body);

            // Attempt to deserialize the JSON response into a Vec<String>
            match serde_json::from_str::<Vec<String>>(&root_body) {
                Ok(strings) => {
                    println!("{}", "now playing:".green().bold());

                    match strings.len() {
                        0 => {
                            println!("  {}", "no songs in the queue.".red().bold());
                        }
                        1 => {
                            println!("    {}", strings[0].yellow().bold());
                        }
                        // because Rust will make us handle the case when there are more than 2 elements
                        // and because we are truncating and only printing two, those cases can collpase
                        // into one instead of `2 => {}; _ => {};`
                        _ => {
                            println!("    {}", strings[0].yellow().bold());
                            println!("{}", "up next:".red().bold());
                            println!("    {}", strings[1].magenta().bold());
                        }
                    }
                }
                Err(e) => {
                    eprintln!("Error: Failed to deserialize root response: {}", e);
                }
            }
        } else {
            eprintln!("Error: Failed to fetch root (HTTP {})", root_response.status());
        }
    } else {
        eprintln!("Error: Failed to fetch status (HTTP {})", response.status());
    }
    Ok(())
}

async fn skip_item(api_hostname: &str) -> Result<(), reqwest::Error> {
    let client = reqwest::Client::new();
    let url = format!("{}/skip", api_hostname);

    let response = client
        .post(&url)
        .header(reqwest::header::CONTENT_LENGTH, "0")
        .send()
        .await?;

    if response.status().is_success() {
        let body = response.text().await?;
        debug!("[+] Item skipped successfully.");

        // Attempt to parse the JSON response into a serde_json::Value
        match serde_json::from_str::<serde_json::Value>(&body) {
            Ok(json) => {
                // Check if the "skipped" and "new" fields exist and are strings
                if let (Some(skipped), Some(new)) = (
                    json["skipped"].as_str(),
                    json["new"].as_str(),
                ) {
                    println!("{}", "[!] SKIPPING SONG".red().bold());
                    println!("    {}", skipped.red());
                    println!("{}", "now playing:".cyan().bold());
                    println!("    {}", new.green().bold());
                } else {
                    eprintln!("Error: Missing or invalid 'skipped' or 'new' fields in JSON response.");
                }
            }
            Err(e) => {
                eprintln!("Error: Failed to parse JSON response: {}", e);
            }
        }
    } else {
        eprintln!("[!] Error: Failed to skip item (HTTP {})", response.status());
    }

    Ok(())
}

async fn playback(
    api_hostname: &str,
    tags_data: &TagsData<>,
) -> Result<(), reqwest::Error> {
    println!("[-] TagsData: {:?}", tags_data);

    let client = reqwest::Client::new();
    let url = format!("{}/tags", api_hostname);

    let response = client
        .post(&url)
        .header(reqwest::header::CONTENT_TYPE, "application/json")
        .body(tags_data.to_json())
        .send()
        .await?;

    if response.status().is_success() {
        println!("[+] Playback Tags updated successfully.");
    } else {
        eprintln!("[!] Error: Failed to update tags (HTTP {})", response.status());
    }

    Ok(())
}

async fn tag(
    api_hostname: &str,
    add_tags: String,
) -> Result<(), reqwest::Error> {
    debug!("[-] Tag Helper: passing thru to perform_tagging()");
    println!("{}{}", "[+] adding tag :".green(), add_tags.green().bold());
    perform_tagging(api_hostname, vec![add_tags], vec![]).await
}

async fn untag(
    api_hostname: &str,
    remove_tags: String,
) -> Result<(), reqwest::Error> {
    debug!("[-] UnTag Helper: passing thru to perform_tagging()");
    println!("{}{}", "[+] removing tag: ".red(), remove_tags.red().bold());
    perform_tagging(api_hostname, vec![], vec![remove_tags]).await
}

async fn perform_tagging(
    api_hostname: &str,
    add_tags: Vec<String>,
    remove_tags: Vec<String>,
) -> Result<(), reqwest::Error> {
    let client = reqwest::Client::new();

    // Make a GET request to the root URL to fetch the "Now Playing" song
    let root_url = format!("{}/", api_hostname);
    let root_response = client.get(&root_url).send().await?;

    println!("{}", "targeting song:".yellow().bold());
    let now_playing: Option<String> = if root_response.status().is_success() {
        let root_body = root_response.text().await?;
        debug!("[?] raw root response body: {}", root_body);

        // Attempt to deserialize the JSON response into a Vec<String>
        match serde_json::from_str::<Vec<String>>(&root_body) {
            Ok(strings) => {
                if let Some(song) = strings.get(0) {
                    Some(song.to_owned())
                } else {
                    None
                }
            }
            Err(e) => {
                eprintln!("Error: Failed to deserialize root response: {}", e);
                None
            }
        }
    } else {
        eprintln!("Error: Failed to fetch root (HTTP {})", root_response.status());
        None
    };

    // gnarly line just to print, someone tell me why this is stupid and wrong
    println!("    {}", now_playing.clone().expect("REASON").to_string().yellow().bold());

    // Create a JSON object representing the request body
    let request_body = serde_json::json!({
        "filename": now_playing,
        "add": add_tags,
        "remove": remove_tags
    });

    let url = format!("{}/song/tags", api_hostname);

    let response = client
        .post(&url)
        .header(reqwest::header::CONTENT_TYPE, "application/json")
        .json(&request_body)
        .send()
        .await?;

    if response.status().is_success() {
        println!("{}", "[+] Tags updated successfully.".green());
    } else {
        eprintln!("[!] Error: Failed to update tags (HTTP {})", response.status());
    }

    Ok(())
}
