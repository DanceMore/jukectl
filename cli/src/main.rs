extern crate clap;
extern crate colored;
extern crate log;
extern crate reqwest;
extern crate serde;
extern crate tokio;

use serde::Deserialize;

use colored::*;

#[allow(unused_imports)]
use log::{debug, error, info, warn};

mod banner;
use crate::banner::print_banner;

mod models;
use crate::models::tags_data::parse_tags_data_from_argv;
use crate::models::tags_data::TagsData;

use clap::{Args, Parser, Subcommand};

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
#[command(propagate_version = true)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Display current status of service
    Status,
    /// Tag the currently playing song
    Tag(TagArgs),
    /// Untag the currently playing song
    Untag(UntagArgs),
    /// Skip the currently playing song
    Skip,
    /// Adjust the jukebox NowPlaying tags
    Playback(PlaybackArgs),
    /// Query the jukebox queue directly
    Queue(QueueArgs),
}

#[derive(Parser)]
struct StatusArgs;

#[derive(Parser)]
struct TagArgs {
    #[clap(help = "Name of the tag", required = true)]
    tag_name: String,
}

#[derive(Parser)]
struct UntagArgs {
    #[clap(help = "Name of the tag", required = true)]
    tag_name: String,
}

#[derive(Parser)]
struct PlaybackArgs {
    #[clap(help = "Tags for playback", required = true)]
    tags: String,
    #[clap(help = "Tags to exclude from playback")]
    not_tags: Option<String>,
}

#[derive(Debug, Args)]
struct QueueArgs {
    #[command(subcommand)]
    command: QueueSubcommand,
}

#[derive(Subcommand, Debug)]
enum QueueSubcommand {
    /// Peek at COUNT from the front of the queue
    Head(QueueHeadArgs),
    /// Peek at COUNT from the end of the queue
    Tail(QueueHeadArgs),
}

#[derive(Parser, Debug)]
struct QueueHeadArgs {
    #[clap(help = "Integer argument for head command")]
    count: i32,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize the logger
    env_logger::init();

    // make compiler warning quiet; it should be getting set or exiting.
    #[allow(unused_assignments)]
    let mut api_hostname = "http://default-api-hostname.com".to_string();

    // Access the JUKECTL_HOST environment variable
    if let Ok(hostname) = std::env::var("JUKECTL_HOST") {
        api_hostname = hostname;
        info!("[-] jukectl API base URL: {}", api_hostname);
    } else {
        eprintln!("Error: JUKECTL_HOST environment variable is not set.");
        std::process::exit(1);
    }

    // clap crate, giving me almost Ruby-Thor library vibes and easy
    // command-line arg parsing :D
    let cli = Cli::parse();

    match cli.command {
        Commands::Status => {
            // Handle status subcommand
            match status(&api_hostname).await {
                Ok(_) => debug!("Status: OK"),
                Err(err) => eprintln!("[!] Error: {}", err),
            }
        }
        Commands::Tag(args) => {
            // Handle tag subcommand
            debug!("Tag an item with name: {:?}", args.tag_name);
            tag(&api_hostname, args.tag_name.to_string()).await?;
        }
        Commands::Untag(args) => {
            debug!("Untag an item with name: {:?}", args.tag_name);
            untag(&api_hostname, args.tag_name.to_string()).await?;
        }
        Commands::Skip => {
            // Handle skip subcommand
            match skip_item(&api_hostname).await {
                Ok(_) => debug!("Skipped item"),
                Err(err) => eprintln!("[!] Error: {}", err),
            }
        }
        Commands::Playback(args) => {
            let not_tags = match args.not_tags {
                Some(tags) => tags,
                None => "".to_string(), // Or use your preferred default value
            };

            let tags_data = parse_tags_data_from_argv(&args.tags, &not_tags);
            match playback(&api_hostname, &tags_data).await {
                Ok(_) => debug!("Playback started with tags: {:?}", tags_data),
                Err(err) => eprintln!("[!] Error: {}", err),
            }
        }

        Commands::Queue(args) => match args.command {
            QueueSubcommand::Head(args) => {
                print_banner();
                debug!("Queue head command with count: {:?}", args.count);
                let queue_data = queue_by_count(&api_hostname, args.count.try_into().unwrap())
                    .await
                    .unwrap_or_default();

                println!(
                    "{}{}",
                    "queue length: ".green(),
                    queue_data.length.to_string().green().bold()
                );

                for (index, song) in queue_data.head.iter().enumerate() {
                    let color = if index % 2 == 0 { "cyan" } else { "magenta" };

                    println!("  {}", song.to_string().color(color));
                }
            }
            QueueSubcommand::Tail(args) => {
                print_banner();
                debug!("Queue tail command with count: {:?}", args.count);
                let queue_data = queue_by_count(&api_hostname, args.count.try_into().unwrap())
                    .await
                    .unwrap_or_default();

                println!(
                    "{}{}",
                    "queue length: ".green(),
                    queue_data.length.to_string().green().bold()
                );

                for (index, song) in queue_data.tail.iter().enumerate() {
                    let color = if index % 2 == 0 { "cyan" } else { "magenta" };

                    println!("  {}", song.to_string().color(color));
                }
            }
        },
    }

    Ok(())
}

async fn queue_by_count(api_hostname: &str, count: usize) -> Result<QueueResponse, reqwest::Error> {
    // Fetch queue status with a count
    let queue_data = get_queue(api_hostname, Some(count))
        .await
        .unwrap_or_default();

    Ok(queue_data)
}

#[derive(Debug, Deserialize, Default)]
struct QueueResponse {
    length: usize,
    head: Vec<String>,
    tail: Vec<String>,
}

async fn get_queue(api_hostname: &str, count: Option<usize>) -> Option<QueueResponse> {
    let client = reqwest::Client::new();

    // Construct the URL with the count parameter
    let url = match count {
        Some(c) => format!("{}/queue?count={}", api_hostname, c),
        None => format!("{}/queue", api_hostname),
    };

    let response = client.get(&url).send().await.ok()?;
    if response.status().is_success() {
        let body_queue = response.text().await.ok()?;
        debug!("[?] raw queue response body: {}", body_queue);

        // Attempt to deserialize the JSON response into QueueResponse
        match serde_json::from_str::<QueueResponse>(&body_queue) {
            Ok(queue_data) => Some(queue_data),
            Err(e) => {
                eprintln!("Error: Failed to deserialize queue response: {}", e);
                Some(QueueResponse {
                    length: 0,
                    head: Vec::new(),
                    tail: Vec::new(),
                })
            }
        }
    } else {
        eprintln!(
            "Error: Failed to fetch queue status (HTTP {})",
            response.status()
        );
        Some(QueueResponse {
            length: 0,
            head: Vec::new(),
            tail: Vec::new(),
        })
    }
}

async fn status(api_hostname: &str) -> Result<(), reqwest::Error> {
    print_banner();

    // Fetch queue status with a count of 3
    match get_queue(api_hostname, Some(3)).await {
        Some(queue_data) => {
            println!(
                "                          {}{}",
                "queue length: ".cyan(),
                queue_data.length.to_string().cyan().bold()
            );
        }
	 None => {
            eprintln!("[!] unable to fetch queue, is your host configuration correct? is the service offline?");
            std::process::exit(1);
        }
    }

    let client = reqwest::Client::new();

    // Make the first GET request to /tags
    let url_tags = format!("{}/tags", api_hostname);
    let response_tags = client.get(&url_tags).send().await?;

    if response_tags.status().is_success() {
        let body_tags = response_tags.text().await?;
        debug!("[?] raw response body: {}", body_tags);

        // Attempt to deserialize the JSON response into TagsData
        match serde_json::from_str::<TagsData>(&body_tags) {
            Ok(tags_data) => {
                println!("{}", "current playback tags:".cyan().bold());
                println!("    {}: {:?}", "any".green().bold(), tags_data.any);
                println!("    {}: {:?}", "not".red().bold(), tags_data.not);
            }
            Err(e) => {
                eprintln!("Error: Failed to deserialize response: {}", e);
            }
        }
    } else {
        eprintln!(
            "Error: Failed to fetch status (HTTP {})",
            response_tags.status()
        );
        return Ok(());
    }

    // Make an additional GET request to the root URL
    let url_root = format!("{}/", api_hostname);
    let response_root = client.get(&url_root).send().await?;

    if response_root.status().is_success() {
        let body_root = response_root.text().await?;
        debug!("[?] raw root response body: {}", body_root);

        // Attempt to deserialize the JSON response into a Vec<String>
        match serde_json::from_str::<Vec<String>>(&body_root) {
            Ok(strings) => {
                println!("{}", "now playing:".green().bold());

                match strings.len() {
                    0 => {
                        println!("  {}", "no songs in the queue.".red().bold());
                    }
                    1 => {
                        println!("    {}", strings[0].yellow().bold());
                    }
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
        eprintln!(
            "Error: Failed to fetch root (HTTP {})",
            response_root.status()
        );
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
                if let (Some(skipped), Some(new)) = (json["skipped"].as_str(), json["new"].as_str())
                {
                    println!("{}", "[!] SKIPPING SONG".red().bold());
                    println!("    {}", skipped.red());
                    println!("{}", "now playing:".cyan().bold());
                    println!("    {}", new.green().bold());
                } else {
                    eprintln!(
                        "Error: Missing or invalid 'skipped' or 'new' fields in JSON response."
                    );
                }
            }
            Err(e) => {
                eprintln!("Error: Failed to parse JSON response: {}", e);
            }
        }
    } else {
        eprintln!(
            "[!] Error: Failed to skip item (HTTP {})",
            response.status()
        );
    }

    Ok(())
}

async fn playback(api_hostname: &str, tags_data: &TagsData) -> Result<(), reqwest::Error> {
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
        eprintln!(
            "[!] Error: Failed to update tags (HTTP {})",
            response.status()
        );
    }

    Ok(())
}

async fn tag(api_hostname: &str, add_tags: String) -> Result<(), reqwest::Error> {
    debug!("[-] Tag Helper: passing thru to perform_tagging()");
    println!("{} {}", "[+] adding tag:".green(), add_tags.green().bold());
    perform_tagging(api_hostname, vec![add_tags], vec![]).await
}

async fn untag(api_hostname: &str, remove_tags: String) -> Result<(), reqwest::Error> {
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
            Ok(strings) => strings.get(0).map(|song| song.to_owned()),
            Err(e) => {
                eprintln!("Error: Failed to deserialize root response: {}", e);
                None
            }
        }
    } else {
        eprintln!(
            "Error: Failed to fetch root (HTTP {})",
            root_response.status()
        );
        None
    };

    // gnarly line just to print, someone tell me why this is stupid and wrong
    println!(
        "    {}",
        now_playing
            .clone()
            .expect("REASON")
            .to_string()
            .yellow()
            .bold()
    );

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
        eprintln!(
            "[!] Error: Failed to update tags (HTTP {})",
            response.status()
        );
    }

    Ok(())
}
