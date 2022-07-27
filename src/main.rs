use std::{env, fs, path::PathBuf, sync::mpsc, thread};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[should_panic]
    fn parse_insuffiecient_args() {
        let args = vec![
            String::from("program_name"),
            String::from("path/to/directory"),
        ];
        // Code below panics as unwrap() is called on the Err variant
        Config::parse(args.into_iter()).unwrap();
    }
}

struct Config {
    path: PathBuf,
    query: String,
}

impl Config {
    fn parse(mut args: impl Iterator<Item = String>) -> Result<Config, &'static str> {
        args.next(); // skip first arguments as it's just a name of the program
        match args.next() {
            Some(path) => match args.next() {
                Some(query) => {
                    let path = match fs::canonicalize(path) {
                        Ok(result) => result,
                        Err(_) => return Err("Given path is not valid"),
                    };
                    Ok(Config { path, query })
                }
                None => return Err("No text to search for. Usage: cargo run -- <PATH> <QUERY>"),
            },
            None => return Err("No file path found. Usage: cargo run -- <PATH> <QUERY>"),
        }
    }
}

/// Recursively searches given path for the files that contain given query and sends them through
/// Sender channel
fn search_in_path(path: &PathBuf, query: String, tx: mpsc::Sender<String>) {
    if path.is_dir() {
        let mut handles = vec![];
        // Here we create inner block so our paths variable goes out of scope and frees resources,
        // preventing from deadlock when we iterate over a big directory and reach maximum amount
        // of open files and all of them are directories (read_dir creates file handle)
        {
            // We need this loop because sometimes there will be an os error 24 (too many open file) when searching in big directories
            // And we will have to wait for some thread to finish and free the resources
            // If we don't use loop here - some directories won't be scanned due to aforementioned
            // error and this will result in an undefined behaviour
            let paths = loop {
                match fs::read_dir(path) {
                    Ok(paths) => break paths,
                    Err(e) => match e.kind() {
                        std::io::ErrorKind::PermissionDenied => {
                            println!("Permission denied: {}", path.to_string_lossy());
                            return;
                        }
                        _ => {
                            // Too many open files -> wait for 1000 nanoseconds and retry
                            if e.to_string().contains("os error 24") {
                                std::thread::sleep(std::time::Duration::from_nanos(1000));
                                continue;
                            } else {
                                println!("Unexpected error during directory iteration: {}", e);
                                return;
                            }
                        }
                    },
                };
            };
            for sub_path in paths {
                let sub_path = match sub_path {
                    Ok(path) => path,
                    Err(e) => {
                        println!(
                            "Unexpected error happened during directory iteration: {}",
                            e
                        );
                        continue;
                    }
                };
                let tx = tx.clone();
                let query = query.clone();
                let handle = thread::spawn(move || search_in_path(&sub_path.path(), query, tx));
                handles.push(handle);
            }
        }
        for handle in handles {
            match handle.join() {
                Ok(_) => (),
                Err(_) => {
                    println!("Searching thread panicked")
                }
            }
        }
    } else {
        // We need this loop because sometimes there will be an os error 24 (too many open file) when searching in big directories
        // And we will have to wait for some thread to finish and free the resources
        // If we don't use loop here - some directories won't be scanned due to aforementioned
        // error and this will result in an undefined behaviour
        let contents = loop {
            match fs::read_to_string(&path) {
                Ok(contents) => break contents,
                Err(e) => {
                    match e.kind() {
                        // There can be a lot of non-utf8 files, so we just skip them
                        std::io::ErrorKind::InvalidData => return,
                        std::io::ErrorKind::PermissionDenied => {
                            println!("Permission denied: {}", path.to_string_lossy());
                            return;
                        }
                        _ => {
                            // Too many open files -> wait for 1000 nanoseconds and retry
                            if e.to_string().contains("os error 24") {
                                std::thread::sleep(std::time::Duration::from_nanos(1000));
                                continue;
                            } else {
                                println!("Unexpected error while opening a file: {}", e);
                                return;
                            }
                        }
                    };
                }
            };
        };
        if contents.contains(query.as_str()) {
            match tx.send(path.to_string_lossy().to_string()) {
                Ok(_) => (),
                Err(_) => println!("Error while sending results from the thread"),
            }
        }
    };
}

fn main() {
    let config = match Config::parse(env::args()) {
        Ok(result) => result,
        Err(e) => {
            println!("Error during argument parsing: {}", e);
            std::process::exit(1);
        }
    };
    let (tx, rx) = mpsc::channel();
    search_in_path(&config.path, config.query, tx);
    for result in rx.iter() {
        println!("{}", result)
    }
}
