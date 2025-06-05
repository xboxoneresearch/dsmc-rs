use std::io::Write;
use indicatif::{ProgressBar, ProgressStyle};
use clap::{Parser, Subcommand};

use dsmc::{DSMC, DSmcError, DSMCFunctions, SMC_NAND_BLOCK_SZ};

const NAND_SIZE: u64 = 5056 * 1024 * 1024; // 5056 MB
const NAND_SECTORS: usize = (NAND_SIZE / SMC_NAND_BLOCK_SZ as u64) as usize; // 10354688 sectors
const NUM_SECTORS_PER_OP: usize = 8;  // physical block size of 0x1000 per transfer to make it safe

#[derive(Subcommand, Debug)]
enum Command {
    /// Read Flash
    Read {
        /// Output file path
        #[arg(short, long)]
        file: String,
        /// Start offset in bytes
        #[arg(short, long, default_value_t = 0)]
        offset: u64,
        /// Number of bytes to read (defaults to entire NAND)
        #[arg(short, long)]
        length: Option<u64>,
    },
    /// Write Flash
    Write {
        /// Input file path
        #[arg(short, long)]
        file: String,
        /// Start offset in bytes
        #[arg(short, long, default_value_t = 0)]
        offset: u64,
    },
    /// Get expected 1SMCBL digest
    Digest,
}


#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// Subcommand
    #[command(subcommand)]
    command: Command,

    /// Safe transfer mode (verification of each read/write transaction)
    #[arg(short, long, default_value_t = false)]
    safe: bool,
}

fn to_hexstr(buf: &[u8]) -> String {
    buf.iter().map(|b| format!("{:02x}", b)).collect()
}


fn handle(args: Args) -> Result<(), DSmcError> {
    let dsmc = DSMC::new().unwrap();

    let version = dsmc.get_interface_version()?;
    println!("dsmcdll version: {}", version);

    if version != 3 {
       return Err(DSmcError::InvalidVersion(version));
    }

    dsmc.initialize(0)?;
    dsmc.begin_programming()?;

    let pb = ProgressBar::new(0);
    pb.set_style(ProgressStyle::default_bar()
        .template("{spinner:.green} [{elapsed_precise}] [{wide_bar:.cyan/blue}] {bytes}/{total_bytes} ({bytes_per_sec}, {eta})")
        .unwrap()
        .progress_chars("#>-"));
    
    dsmc.set_safe_transfer_mode(args.safe)?;

    match args.command {
        Command::Read { file, offset, length } => {
            let start_sector = (offset / SMC_NAND_BLOCK_SZ as u64) as usize;
            let num_sectors = match length {
                Some(len) => len.div_ceil(SMC_NAND_BLOCK_SZ as usize) as usize,
                None => NAND_SECTORS - start_sector,
            };

            if start_sector >= NAND_SECTORS {
                eprintln!("Error: Start offset exceeds NAND size");
                return Ok(());
            }
            if start_sector + num_sectors > NAND_SECTORS {
                eprintln!("Error: Read operation would exceed NAND size");
                return Ok(());
            }

            let mut file = std::fs::File::create(file).unwrap();
            let total_bytes = (num_sectors * SMC_NAND_BLOCK_SZ as usize) as u64;
            pb.set_length(total_bytes);
            
            for sector in (start_sector..start_sector + num_sectors).step_by(NUM_SECTORS_PER_OP) {
                let sectors_to_read = std::cmp::min(
                    NUM_SECTORS_PER_OP,
                    start_sector + num_sectors - sector
                );
                let read = dsmc.block_read(sector as i32, sectors_to_read as i32)?;
                file.write_all(&read).unwrap();
                pb.set_position(((sector - start_sector) * SMC_NAND_BLOCK_SZ as usize) as u64);
            }
            pb.finish();
        },
        Command::Write { file, offset } => {
            let start_sector = (offset / SMC_NAND_BLOCK_SZ as u64) as usize;
            let data = std::fs::read(file).unwrap();
            let num_sectors = data.len().div_ceil(SMC_NAND_BLOCK_SZ as usize) as usize;

            if start_sector >= NAND_SECTORS {
                eprintln!("Error: Start offset exceeds NAND size");
                return Ok(());
            }
            if start_sector + num_sectors > NAND_SECTORS {
                eprintln!("Error: Write operation would exceed NAND size");
                return Ok(());
            }

            pb.set_length(data.len() as u64);

            for (i, chunk) in data.chunks(NUM_SECTORS_PER_OP * SMC_NAND_BLOCK_SZ as usize).enumerate() {
                let sector = start_sector + (i * NUM_SECTORS_PER_OP);
                dsmc.block_write(sector as i32, chunk)?;
                pb.set_position((i * NUM_SECTORS_PER_OP * SMC_NAND_BLOCK_SZ as usize) as u64);
            }
            pb.finish();
        },
        Command::Digest => {
            println!("Before calling");
            let digest = dsmc.get_exp_digest_1smcbl()?;
            println!("Exected 1SMCBL Digest: {}", to_hexstr(&digest));
        }
    }

    dsmc.end_programming()?;
    dsmc.release();

    Ok(())
}

fn main() {
    let args = Args::parse();

    if let Err(err) = handle(args) {
        eprintln!("[ERR] {err}");
    }
}
