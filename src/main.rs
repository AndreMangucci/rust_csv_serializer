use csv::{ByteRecord, ReaderBuilder, StringRecord};
use encoding_rs::WINDOWS_1252 as ENCODING;
use serde::Deserialize;
use std::error::Error;
use std::fs::File;
use std::io::{self, BufWriter, Write};
// use std::result;

#[derive(Debug, Deserialize)]
struct Row<'a> {
    _id: u32,
    date: &'a str,
    _name: String,
    _1: &'a str,
    _2: &'a str,
    _status: &'a str,
    _3: &'a str,
    worker_id: Option<i32>,
    _device: &'a str,
    _local: &'a str,
}

fn main() -> Result<(), Box<dyn Error>> {
    // Open the CSV file
    let file = File::open("data/example.csv")?;

    // Create a CSV reader with no headers
    let mut rdr = ReaderBuilder::new().has_headers(true).from_reader(file);

    // Create a buffer for each record
    let mut record = ByteRecord::new();

    let mut lines: Vec<String> = vec![];

    // Read each record as raw bytes
    while rdr.read_byte_record(&mut record)? {
        // Decode each field from WINDOWS_1252 to UTF-8
        let utf8_record: Vec<String> = record
            .iter()
            .map(|field| {
                // Decode using WINDOWS_1252
                let (decoded, _, had_errors) = ENCODING.decode(field);
                if had_errors {
                    eprintln!("Warning: Encoding errors detected in field: {:?}", field);
                }
                decoded.into_owned()
            })
            .collect();

        // Convert the Vec<String> to a StringRecord
        let string_record = StringRecord::from(utf8_record);

        // Deserialize the record into a Row struct
        let row: Row = string_record.deserialize(None)?;

        // Formating Data

        // RE
        let alignment = 15;
        // let f_worker_id = format!("{:0fill$}", row.worker_id, fill = alignment);
        let f_worker_id = match row.worker_id {
            Some(id) => format!("{:0fill$}", id, fill = alignment),
            None => {
                eprintln!("Skipping row with missing worker_id.");
                continue;
            }
        };

        // Date
        let date = row.date;
        let f_date = format!("{}{}{}", &date[..2], &date[3..5], &date[8..10]);

        // Time
        let f_time = format!("{}{}{}", &date[11..13], &date[14..16], &date[17..19]);

        // Final row
        let end_row = String::from("100200");
        // println!("{f_worker_id}{f_date}{f_time}{end_row}");

        //  Write File
        let f_row = format!("{f_worker_id}{f_date}{f_time}{end_row}");
        lines.push(f_row);
    }

    let _result = write_lines_to_file("output.txt", &lines);

    Ok(())
}

fn write_lines_to_file(filename: &str, lines: &[String]) -> io::Result<()> {
    let file = File::create(filename)?;
    let mut writer = BufWriter::new(file);

    for line in lines {
        writeln!(writer, "{}", line)?;
    }

    writer.flush()?; // Ensure all data is flushed to the file
    Ok(())
}
