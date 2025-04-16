use std::fs::File;
use std::{env, fs, io::Read, process::exit};
use std::io::{self, copy, Write};
use home;
use std::path::Path;
use reqwest::blocking::{get, Response};
use sha2::{Digest, Sha512};

struct ModData {
    path: String,
    hash: String,
    download: String,
}




fn help(args: Vec<String>) {
    println!("
    Usage: {} <path-to-mrpack> <path-to-server>
    ", args[0])
}



fn prompt(message: &str) -> bool {
    loop {
        print!("{} (y/n): ", message);
        io::stdout().flush().unwrap(); // Ensure the prompt shows up immediately

        let mut input = String::new();
        io::stdin().read_line(&mut input).unwrap();

        match input.trim().to_lowercase().as_str() {
            "y" | "yes" => return true,
            "n" | "no" => return false,
            _ => {
                println!("Invalid input. Please answer 'y' or 'n'.");
            }
        }
    }
}



fn expand_tilde(path: &str) -> String {
    if path.starts_with("~") {
        let home_dir = home::home_dir().expect("Could not find home directory");
        let expanded = home_dir.join(&path[1..]);
        return expanded.to_str().unwrap_or_default().to_string();
    }
    path.to_string()
}



fn get_sha512_from_response(response: &mut Response) -> Result<String, Box<dyn std::error::Error>> {
    let mut hasher = Sha512::new();
    let mut buffer = [0u8; 4096]; // A buffer to read chunks

    while let Ok(bytes_read) = response.read(&mut buffer) {
        if bytes_read == 0 {
            break; // No more data to read
        }
        hasher.update(&buffer[..bytes_read]);
    }

    // Finalize the hash computation
    let hash = hasher.finalize();
    let hex_hash = hex::encode(hash);

    Ok(hex_hash)
}



fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 3 { 
        help(args);
        exit(1);
    }

    // arg2:
    // the path to server
    let expanded_server_path = expand_tilde(args[2].as_str());
    let server_path = Path::new(expanded_server_path.as_str());

    // arg1: 
    // the path to .mrpack
    let mrpack_path: String = expand_tilde(args[1].as_str());

    println!("Opening mrpack");
    let mrpack_file = fs::File::open(mrpack_path.as_str()).expect("Failed to open mrpack, check so filepath is right!");
    println!("Extracting mrpack");
    let mut archive = zip::ZipArchive::new(mrpack_file).expect("Failed to unzip mrpack file!");
    let mut index = archive.by_name("modrinth.index.json").expect("No `modrinth.index.json` found in mrpack file!");

    println!("Getting mrpack `modrinth.index.json` content");
    let mut content = String::new();
    index.read_to_string(&mut content).expect("Failed to read `modrinth.index.json`!");
    
    println!("Parsing `modrinth.index.json`");
    let parsed: json::JsonValue = json::parse(content.as_str()).expect("Failed to parse `modrinth.index.json`!");
    let mut mods: Vec<ModData> = Vec::new();

    println!("Enterperting parsed data");
    for raw_mod_data in parsed["files"].members() {
        if raw_mod_data["env"]["server"].as_str().unwrap() == "required" {
            mods.push(ModData {
                path: raw_mod_data["path"].as_str().unwrap().to_string(),
                hash: raw_mod_data["hashes"]["sha512"].as_str().unwrap().to_string(),
                download: raw_mod_data["downloads"][0].as_str().unwrap().to_string(),
            });
        }
    }

    if ! prompt(format!("\nDo you want to install {} mods at {}?", mods.len(), server_path.to_str().unwrap()).as_str()) {
        exit(0);
    }

    // start downloading the mods'
    println!("Starting downloads: ");
    for mod_data in mods.iter() {
        let mut response = get(mod_data.download.as_str()).expect("Failed to download file!");
        let response_hash = get_sha512_from_response(&mut response).expect("Failed to get hash from downloaded response!");

        if response_hash != mod_data.hash {
            println!("    Hash of downloaded file didn't match modpack hash: {}   skipping file", mod_data.path);
            continue;
        }

        let full_path = server_path.join(mod_data.path.as_str());
        let mut file = File::create(full_path).expect(format!("Failed to create file `{}`", mod_data.path).as_str());
        copy(&mut response, &mut file).expect(format!("Failed to write to file `{}`", mod_data.path).as_str());

        println!("    Successfully downloaded mod `{}`", mod_data.path);
    }

    println!("DONE!");
}
