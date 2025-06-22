use std::{
    fs::File,
    io::{BufReader, BufWriter, Read},
    path::Path,
};

pub fn compress_to_zstd(input: &Path, output: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let input_file = File::open(input)?;
    let buffered_reader = BufReader::new(input_file);

    let output_file = File::create(output)?;
    let buffered_writer = BufWriter::new(output_file);

    let mut encoder = zstd::stream::Encoder::new(buffered_writer, 3)?;
    std::io::copy(&mut buffered_reader.take(u64::MAX), &mut encoder)?;
    encoder.finish()?;

    Ok(())
}
