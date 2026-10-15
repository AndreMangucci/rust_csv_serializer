use chrono::{Local, NaiveDateTime};
use csv::{ByteRecord, ReaderBuilder, StringRecord, Writer};
use encoding_rs::WINDOWS_1252 as ENCODING;
use serde::Deserialize;
use std::error::Error;
use std::fs::{self, File};
use std::io::{self, BufWriter, Write};

const EXPECTED_COLS: usize = 10;

#[derive(Debug, Deserialize)]
struct Row<'a> {
    _id: &'a str,
    date: &'a str,
    _name: String,
    _1: &'a str,
    _2: &'a str,
    _status: &'a str,
    _3: &'a str,
    worker_id: &'a str, // parsed manually so we can report the raw value
    _device: &'a str,
    _local: &'a str,
}

fn log_error(
    report: &mut Writer<File>,
    file: &str,
    line: u64,
    kind: &str,
    obs: &str,
) -> csv::Result<()> {
    eprintln!("[{file}:{line}] {kind} - {obs}");
    report.write_record([file, &line.to_string(), kind, obs])
}

fn main() -> Result<(), Box<dyn Error>> {
    fs::create_dir_all("./data_out")?;

    // Report file, one per execution
    let report_name = format!(
        "data_out/relatorio_{}.csv",
        Local::now().format("%Y%m%d_%H%M%S")
    );
    let mut report = Writer::from_path(&report_name)?;
    report.write_record(["nome_arquivo", "linha_erro", "tipo_erro", "observacao"])?;

    let paths = fs::read_dir("./data_in/")?;

    for path in paths {
        let entry = path?;
        let file = File::open(entry.path())?;

        let file_name_osstr = entry.file_name();
        let file_name = file_name_osstr.to_str().unwrap();

        println!("Processing file: {:?}", file_name);

        // flexible(true): don't abort on rows with a wrong column count,
        // we check it ourselves and report it.
        let mut rdr = ReaderBuilder::new()
            .has_headers(true)
            .flexible(true)
            .from_reader(file);

        let mut record = ByteRecord::new();
        let mut lines: Vec<String> = vec![];

        while rdr.read_byte_record(&mut record)? {
            let line = record.position().map(|p| p.line()).unwrap_or(0);

            // Wrong number of columns
            if record.len() != EXPECTED_COLS {
                log_error(
                    &mut report,
                    file_name,
                    line,
                    "numero de colunas incorreto",
                    &format!("esperado {}, encontrado {}", EXPECTED_COLS, record.len()),
                )?;
                continue;
            }

            // Decode WINDOWS_1252 -> UTF-8
            let mut encoding_problem = false;
            let utf8_record: Vec<String> = record
                .iter()
                .map(|field| {
                    let (decoded, _, had_errors) = ENCODING.decode(field);
                    if had_errors {
                        encoding_problem = true;
                    }
                    decoded.into_owned()
                })
                .collect();

            let string_record = StringRecord::from(utf8_record);

            if encoding_problem {
                log_error(
                    &mut report,
                    file_name,
                    line,
                    "erro de encoding",
                    &format!("{:?}", string_record),
                )?;
            }

            let row: Row = match string_record.deserialize(None) {
                Ok(r) => r,
                Err(e) => {
                    log_error(
                        &mut report,
                        file_name,
                        line,
                        "linha ilegivel",
                        &e.to_string(),
                    )?;
                    continue;
                }
            };

            let mut had_error = false;

            // Worker ID
            let worker_id = row.worker_id.trim();
            let f_worker_id = if worker_id.is_empty() {
                log_error(&mut report, file_name, line, "sem ID", &row._name)?;
                had_error = true;
                None
            } else {
                match worker_id.parse::<i32>() {
                    Ok(id) => Some(format!("{:015}", id)),
                    Err(_) => {
                        log_error(
                            &mut report,
                            file_name,
                            line,
                            "ID invalido",
                            &format!("valor: '{}' | nome: {}", worker_id, row._name),
                        )?;
                        had_error = true;
                        None
                    }
                }
            };

            // Date (dd/mm/yyyy hh:mm:ss)
            let parsed_date =
                match NaiveDateTime::parse_from_str(row.date.trim(), "%d/%m/%Y %H:%M:%S") {
                    Ok(d) => Some(d),
                    Err(_) => {
                        log_error(
                            &mut report,
                            file_name,
                            line,
                            "data invalida",
                            &format!("valor nao convertido: '{}'", row.date),
                        )?;
                        had_error = true;
                        None
                    }
                };

            if had_error {
                continue;
            }

            let (f_worker_id, dt) = (f_worker_id.unwrap(), parsed_date.unwrap());
            let f_date = dt.format("%d%m%y");
            let f_time = dt.format("%H%M%S");
            let end_row = "100200";

            lines.push(format!("{f_worker_id}{f_date}{f_time}{end_row}"));
        }

        let new_file_name = format!("data_out/{}", file_name.replace(".csv", ".txt"));
        println!("Writing to file: {:?}", new_file_name);

        if let Err(e) = write_lines_to_file(&new_file_name, &lines) {
            log_error(&mut report, file_name, 0, "erro de escrita", &e.to_string())?;
        }
    }

    report.flush()?;
    println!("Report written to: {}", report_name);
    Ok(())
}

fn write_lines_to_file(filename: &str, lines: &[String]) -> io::Result<()> {
    let file = File::create(filename)?;
    let mut writer = BufWriter::new(file);
    for line in lines {
        writeln!(writer, "{}", line)?;
    }
    writer.flush()?;
    Ok(())
}
