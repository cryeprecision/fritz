use std::path::{Path, PathBuf};

use anyhow::Context;
use chrono::TimeZone;
use fritz_app::db;
use structopt::StructOpt;

#[derive(Debug, serde::Deserialize)]
struct Log {
    id: i64,
    datetime: i64,
    message: String,
    message_id: i64,
    category_id: i64,
    repetition_datetime: Option<i64>,
    repetition_count: Option<i64>,
}

#[derive(Debug, serde::Deserialize)]
struct Request {
    id: i64,
    datetime: i64,
    name: String,
    url: String,
    method: String,
    duration_ms: i64,
    response_code: Option<i64>,
    session_id: Option<String>,
}

#[derive(Debug, serde::Deserialize)]
struct Update {
    id: i64,
    datetime: i64,
    upserted_rows: i64,
}

fn timestamp_to_datetime(timestamp: i64) -> anyhow::Result<chrono::DateTime<chrono::Utc>> {
    chrono::Utc
        .timestamp_millis_opt(timestamp)
        .single()
        .context("Invalid timestamp")
}

impl TryFrom<Log> for db::Log {
    type Error = anyhow::Error;
    fn try_from(log: Log) -> anyhow::Result<Self> {
        Ok(db::Log {
            id: Some(log.id),
            datetime: timestamp_to_datetime(log.datetime)?,
            message: log.message,
            message_id: log.message_id,
            category_id: log.category_id,
            repetition_datetime: match log.repetition_datetime {
                Some(timestamp) => Some(timestamp_to_datetime(timestamp)?),
                None => None,
            },
            repetition_count: log.repetition_count,
        })
    }
}

impl TryFrom<Request> for db::Request {
    type Error = anyhow::Error;
    fn try_from(request: Request) -> anyhow::Result<Self> {
        Ok(db::Request {
            id: Some(request.id),
            datetime: timestamp_to_datetime(request.datetime)?,
            name: request.name,
            url: request.url,
            method: request.method,
            duration_ms: request.duration_ms,
            response_code: request.response_code,
            session_id: request.session_id,
        })
    }
}

impl TryFrom<Update> for db::Update {
    type Error = anyhow::Error;
    fn try_from(update: Update) -> anyhow::Result<Self> {
        Ok(db::Update {
            id: Some(update.id),
            datetime: timestamp_to_datetime(update.datetime)?,
            upserted_rows: update.upserted_rows,
        })
    }
}

#[derive(Debug, StructOpt)]
struct Opt {
    /// Input dir
    #[structopt(parse(from_os_str), long = "input-dir")]
    input_dir: PathBuf,

    /// Output dir
    #[structopt(parse(from_os_str), long = "output-dir")]
    output_dir: PathBuf,
}

fn converter<T, U>(input: &Path, output: &Path) -> anyhow::Result<()>
where
    U: TryFrom<T, Error = anyhow::Error> + serde::Serialize,
    T: serde::de::DeserializeOwned,
{
    // reader
    let in_file = std::fs::OpenOptions::new()
        .read(true)
        .open(input)
        .context("failed to open input file for reading")?;
    let mut reader = csv::Reader::from_reader(in_file);
    let in_iter = reader.deserialize::<T>().map(|row| {
        U::try_from(row.context("failed to deserialize row")?)
            .context("failed to convert row from T to U")
    });

    // writer
    let out_file = std::fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(output)
        .context("failed to open output file for writing")?;
    let out_file_buf = std::io::BufWriter::with_capacity(32 * 1024, out_file);
    let mut writer = csv::Writer::from_writer(out_file_buf);

    in_iter.enumerate().try_for_each(|(i, row)| {
        writer
            .serialize(row.with_context(|| format!("on row {i}"))?)
            .with_context(|| format!("failed to write row {i}"))
    })?;

    Ok(())
}

fn main() -> anyhow::Result<()> {
    let opt = Opt::from_args();

    if !std::fs::metadata(&opt.input_dir).map_or(false, |m| m.is_dir()) {
        anyhow::bail!("Input dir is not a directory");
    }
    if !std::fs::metadata(&opt.output_dir).map_or(false, |m| m.is_dir()) {
        anyhow::bail!("Output dir is not a directory");
    }

    converter::<Log, db::Log>(
        &opt.input_dir.join("logs.csv"),
        &opt.output_dir.join("logs.csv"),
    )?;

    converter::<Request, db::Request>(
        &opt.input_dir.join("requests.csv"),
        &opt.output_dir.join("requests.csv"),
    )?;

    converter::<Update, db::Update>(
        &opt.input_dir.join("updates.csv"),
        &opt.output_dir.join("updates.csv"),
    )?;

    Ok(())
}
