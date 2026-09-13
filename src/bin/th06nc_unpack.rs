use std::fs;
use std::io::Read;
use std::fs::File;
use std::io::{BufReader, BufWriter, Seek, SeekFrom};
use std::path::PathBuf;

use clap::Parser;
use anyhow::{anyhow, Result};
use th06nc_unpack::encrytion::{key_gen, XorReader};
use th06nc_unpack::models::{Header, Storage};
use th06nc_unpack::utils::archive_key;

#[derive(Parser, Debug)]
#[command(name = "th06nc_unpack", version, about = "东方红魔乡新典解包工具")]
struct Args {
    /// 输入路径（必填）
    input: PathBuf,

    /// 输出路径（可选）
    output: Option<PathBuf>,
}

fn main() -> Result<()>
{
    let args = Args::parse();

    let input = args.input;
    let output_ = args.output;
    let output;

    if !input.exists()
    {
        return Err(anyhow!("路径不存在！"));
    }

    if !input.is_file()
    {
        return Err(anyhow!("输入路径必须为文件！"))
    }

    if output_.is_none()
    {
        output = PathBuf::from(archive_key(&input));
    }
    else
    {
        output = output_.unwrap();
    }

    let file = File::open(&input)?;
    let mut reader = BufReader::with_capacity(64 * 1024, file);
    let header = Header::read(&mut reader, &archive_key(&input))?;
    fs::create_dir_all(&output)?;

    for i in header.entries
    {
        let offset = i.offset;
        reader.seek(SeekFrom::Start(offset))?;

        let mut src = XorReader::new(
            (&mut reader).take(i.stored_size),
            key_gen(i.key),
        );

        let output_file = output.join(i.name.as_str());

        let mut out = BufWriter::new(File::create(&output_file)?);
        match i.storage() {
            Storage::Stored => { std::io::copy(&mut src, &mut out)?; }
            Storage::Zstd   => { zstd::stream::copy_decode(src, &mut out)?; }
        }
        println!("解包 {} -> {:?}", &i.name, &output_file);
    }

    Ok(())
}