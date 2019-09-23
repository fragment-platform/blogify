use promptly::{prompt};
use std::fs;
use std::io::{BufReader, Error, Read, Write};
use std::path::{Path, PathBuf};

use chrono::{DateTime, Utc};
use serde::{Serialize};
use structopt::StructOpt;
use toml;
use zip::write::{FileOptions, ZipWriter};

use sha2::{Digest, Sha256};

#[derive(StructOpt)]
#[structopt(name = "blogify")]
#[structopt(about = "make your blog!")]
enum Blogify {
    Init {},
    Post {
        /// Post's input .html file
        #[structopt(parse(from_os_str))]
        post: PathBuf,

        /// Additional assets
        #[structopt(name = "ASSETS", parse(from_os_str))]
        assets: Vec<PathBuf>,
    },
    Hash {
        #[structopt(parse(from_os_str))]
        post: PathBuf,
    },
    Sign {},
    Verify {
        #[structopt(parse(from_os_str))]
        post: PathBuf,
    },
}

#[derive(Serialize)]
struct Meta {
    name: String,
    slug: String,
    published: DateTime<Utc>,
}

fn main() -> Result<(), Error> {
    let blog = Blogify::from_args();

    match blog {
        Blogify::Init {} => {}
        Blogify::Post { post, assets } => {
            let name: String = prompt("Enter the post title");
            let slug: String = prompt("enter_a_slug_like_this");

            let meta = Meta {
                name,
                slug,
                published: Utc::now(),
            };

            // TODO this should be whatever canonical output directory we have
            match fs::create_dir("demo") {
                Ok(()) => println!("Creating demo dir"),
                _ => {}
            }

            let file_name = "demo/".to_owned() + &meta.slug + ".post";
            let file = fs::File::create(&file_name).unwrap();
            let mut zip = ZipWriter::new(file);

            // We use Stored because we don't want compression
            let options = FileOptions::default().compression_method(zip::CompressionMethod::Stored);

            // Create toml
            println!("Adding meta.toml");
            let meta_toml = toml::to_string(&meta).unwrap();
            zip.start_file("meta.toml", options)?;
            zip.write_all(meta_toml.as_bytes())?;

            let mut buffer = Vec::new();

            // Add the .html file to the root of the zip
            add_file(&post, None, &mut zip, &mut buffer)?;

            // Add each (optional) asset to the assets folder
            // TODO: I guess we need to rewrite the references in the html? uh oh!
            for asset in assets {
                add_file(&asset, Some("assets/"), &mut zip, &mut buffer)?;
            }

            zip.finish()?;
        }
        Blogify::Hash { post } => {
            hash_post(&post)?;
        }
        Blogify::Sign {} => {}
        Blogify::Verify { post } => {}
    }

    Ok(())
}

fn hash_post(post: &PathBuf) -> Result<(), Error> {
    use data_encoding::HEXLOWER;
    // Open up the zip file and read all the bytes
    let zip_file = fs::File::open(&post)?;
    let reader = BufReader::new(zip_file);

    let mut archive = zip::ZipArchive::new(reader)?;

    let mut hasher = Sha256::new();

    for i in 0..archive.len() {
        let file = archive.by_index(i).unwrap();
        let outpath = file.sanitized_name();
        println!("Adding {} to buffer", outpath.display());
        let buffer = file.bytes().collect::<Result<Vec<u8>, _>>();
        hasher.input(&buffer.unwrap());
    }

    // Hash the bytes
    let result = hasher.result();
    println!("{}", HEXLOWER.encode(result.as_ref()));

    Ok(())
}

fn add_file(
    path: &Path,
    prefix: Option<&str>,
    zip: &mut ZipWriter<fs::File>,
    buffer: &mut Vec<u8>,
) -> Result<(), Error> {
    println!("Adding {}", path.display());
    let options = FileOptions::default().compression_method(zip::CompressionMethod::Stored);

    let mut new_path;
    if let Some(prefix) = prefix {
        new_path = PathBuf::from(prefix);
        new_path.push(path);
    } else {
        new_path = path.to_path_buf();
    }

    zip.start_file_from_path(&new_path, options)?;

    let mut f = fs::File::open(&path)?;
    f.read_to_end(buffer)?;
    zip.write_all(&*buffer)?;
    buffer.clear();
    Ok(())
}