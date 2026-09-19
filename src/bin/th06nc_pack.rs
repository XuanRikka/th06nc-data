use std::io::{BufReader, Write};
use std::fs::File;
use std::io::{BufWriter, Seek, SeekFrom};
use std::path::PathBuf;

use clap::Parser;
use anyhow::{anyhow, Result};
use byteorder::{LittleEndian, WriteBytesExt};
use crc32fast::hash;
use zstd::stream::read::Encoder;

use th06nc_data::encrytion::{key_gen, XorReader, apply_key, XorWriter};
use th06nc_data::models::{Storage, MAIGC, ENTRY_LENGTH, VER0102_ENTRY_LENGTH, Entry, VER0102_DATA};
use th06nc_data::padding_to_16;
use th06nc_data::utils::{archive_key, walk_files};

#[derive(Parser, Debug)]
#[command(name = "th06nc_pack", version, about = "东方红魔乡新典封包工具")]
struct Args {
    /// 输入路径
    input: PathBuf,

    /// 输出路径
    output: PathBuf,

    /// 压缩等级
    /// 范围: -131072 - 22
    #[arg(long, default_value_t = 6)]
    compress_level: i32,

    /// 不压缩
    #[arg(long)]
    no_compress: bool,

    /// 不在末尾追加ver0102
    #[arg(long)]
    no_ver0102: bool,
}

fn main() -> Result<()>
{
    let args = Args::parse();

    let args_input = args.input;
    let args_output = args.output;

    if !args_input.exists()
    {
        return Err(anyhow!("输入路径不存在！"));
    }

    if !args_input.is_dir()
    {
        return Err(anyhow!("输入路径必须是目录"));
    }

    let mut files = Vec::new();
    for item in walk_files(&args_input) {
        if let Ok(path) = item {
            let filename = path.file_name().unwrap();
            if filename.is_ascii()
            {
                files.push(path);
            }
            else
            {
                println!("文件 {} 文件名中存在非ASCII字符，故跳过", path.to_string_lossy())
            }
        }
        else
        {
            println!("遍历文件时出现错误！{}", item.err().unwrap())
        }
    }

    let output_file_raw = File::create(&args_output)?;
    let mut output_file = BufWriter::new(output_file_raw);

    output_file.write_all(MAIGC)?;

    let mut index_length = files
        .iter()
        .map(|x| x.file_name().unwrap().len()+ENTRY_LENGTH)
        .sum::<usize>();

    if !args.no_ver0102
    {
        index_length = index_length + VER0102_ENTRY_LENGTH;
    }

    output_file.write_u32::<LittleEndian>(index_length as u32)?;
    let index_offset = output_file.stream_position()?;
    let data_start_temp = 8 + (index_length as u64);
    output_file.seek(SeekFrom::Start(data_start_temp + padding_to_16!(data_start_temp)))?;

    let mut entries = Vec::new();

    for path in files
    {
        let offset_raw = output_file.stream_position()?;
        let offset = offset_raw + padding_to_16!(offset_raw);
        output_file.seek(SeekFrom::Start(offset))?;

        let seed = getrandom::u32()?;
        let key = key_gen(seed);
        let file = File::open(&path)?;
        let file_size = file.metadata()?.len();
        let file_name = path.file_name().unwrap().to_str().unwrap();

        let storage_type;
        let stored_size;
        if !args.no_compress && file_name != "ver0102.dat"
        {
            let zstd_stream = Encoder::new(file, args.compress_level)?;
            let mut xor_steam = XorReader::new(zstd_stream, key);
            stored_size = std::io::copy(&mut xor_steam, &mut output_file)?;
            storage_type = Storage::Zstd;
        }
        else
        {
            let mut xor_steam = XorReader::new(BufReader::new(file), key);
            stored_size = std::io::copy(&mut xor_steam, &mut output_file)?;
            storage_type = Storage::Stored;
        }

        let entry = Entry::new(
            storage_type,
            seed,
            file_size,
            stored_size,
            offset,
            file_name
        );
        entries.push(entry);
        output_file.flush()?;
    }

    if !args.no_ver0102
    {
        let ver0102_seed = getrandom::u32()?;
        let mut ver0102_data = VER0102_DATA.to_vec();
        apply_key(&mut ver0102_data, &key_gen(ver0102_seed));

        let offset_raw = output_file.stream_position()?;
        let offset = offset_raw + padding_to_16!(offset_raw);
        output_file.seek(SeekFrom::Start(offset))?;

        entries.push(Entry::new(
            Storage::Stored, ver0102_seed,
            VER0102_DATA.len() as u64, VER0102_DATA.len() as u64,
            offset, "ver0102.dat",
        ));
        output_file.write_all(&ver0102_data)?;
    }

    let dat_name = archive_key(&args_output);
    let header_key = key_gen(hash(dat_name.as_bytes()));

    output_file.seek(SeekFrom::Start(index_offset))?;

    let mut output_file = XorWriter::new(output_file, header_key);
    for entry in entries
    {
        entry.write(&mut output_file)?;
    }
    output_file.flush()?;

    let mut output_file = output_file.into_inner();
    output_file.seek(SeekFrom::End(0))?;
    let pos = output_file.stream_position()?;
    output_file.write_all(&vec![0u8; (padding_to_16!(pos) as usize)])?;
    output_file.flush()?;

    Ok(())
}
