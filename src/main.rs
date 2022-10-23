#![allow(dead_code)]

//! TODO: report on failed parsing (miette)
//! TODO: session table, session start time, start -> {success, error}
//! TODO: limit vegetable pulls to my character (DJAN)
//! TODO: record vegetable pulls in database
//! TODO: update README
//! TODO: get sample over time duration?
//! TODO: compare stitch pulls against EV for sample
//! TODO: d3 visualization for time series
//! - <https://observablehq.com/@d3/stacked-bar-chart>
//! - <https://plotly.com/javascript/histograms/#colored-and-styled-histograms>

use peppi::model::enums::item::Type;
use peppi::model::frame::Frame;
use peppi::model::game::Frames;
use peppi::model::item::Item;
use peppi::model::metadata::Player;
use peppi::model::primitives::Port;
use sqlx::migrate::Migrator;
use sqlx::sqlite::SqlitePoolOptions;
use std::collections::HashMap;
use std::path::PathBuf;
use std::{env, fs, io};
use thiserror::Error;
use walkdir::WalkDir;

static MIGRATOR: Migrator = sqlx::migrate!("db/migrations");
const DATABASE_URL: &str = "sqlite://turnips.db";

// TODO: error handling for database
/// Errors handled by turnip-counter.
#[derive(Debug, Error)]
enum TurnipError {
    #[error("no misc data")]
    MissingMisc,

    #[error("only supports 2 player games")]
    WrongNumberPlayers,

    #[error("not a peach item")]
    NotPeachItem,

    #[error("unowned item??")]
    MissingOwner,

    #[error("???")]
    OwnerNotPlayer,

    #[error("failed to open replay file")]
    OpenFailed(std::io::Error),

    #[error("failed to parse")]
    ParseFailed(peppi::ParseError),
}

type App<T> = Result<T, TurnipError>;

/// Possible peach items.
/// Turnip faces are taken from second byte of misc field.
#[derive(Debug, Clone, Copy)]
enum PeachItem {
    /// `(0..4).contains(misc[1])`
    NormalTurnip,
    /// `misc[1] == 5`
    WinkyTurnip,
    /// `misc[1] == 6`
    DotEyesTurnip,
    /// `misc[1] == 7`
    StitchTurnip,
    /// Electric.
    Beamsword,
    /// Explosive!
    Bobomb,
    /// Friendly :)
    MrSaturn,
}

#[derive(Debug, Clone, Copy)]
struct ItemData {
    /// What kind of item?
    kind: PeachItem,
    /// The first frame the item was seen.
    frame: i32,
    /// The initial owner of the item.
    owner: Port,
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
enum ItemState {
    Unknown(u8),
}

#[derive(Debug, Clone, Copy)]
struct StateSnapshot {
    frame: i32,
    state: ItemState,
    owner: Port,
}

#[derive(Debug)]
struct ItemHistory {
    data: ItemData,
    history: Vec<StateSnapshot>,
}

/// Index pulled turnips by item ID.
type ItemLog = HashMap<u32, ItemHistory>;

/// Check to see if the item is a turnip.
fn parse_item(frame: i32, item: &Item) -> App<(u32, ItemData, StateSnapshot)> {
    let kind: PeachItem = match item.r#type {
        Type::BOB_OMB => PeachItem::Bobomb,
        Type::BEAM_SWORD => PeachItem::Beamsword,
        Type::MR_SATURN => PeachItem::MrSaturn,
        Type::PEACH_TURNIP => {
            let face_byte = item.misc.expect("no misc data")[1];
            match face_byte {
                0 => PeachItem::NormalTurnip,
                1 => PeachItem::NormalTurnip,
                2 => PeachItem::NormalTurnip,
                3 => PeachItem::NormalTurnip,
                4 => PeachItem::NormalTurnip,
                5 => PeachItem::WinkyTurnip,
                6 => PeachItem::DotEyesTurnip,
                7 => PeachItem::StitchTurnip,
                _ => panic!("unknown turnip face"),
            }
        }
        _ => Err(TurnipError::NotPeachItem)?,
    };

    let owner: Port = item
        .owner
        .ok_or(TurnipError::MissingOwner)?
        .ok_or(TurnipError::OwnerNotPlayer)?;

    // we don't know what item states are yet
    let state = ItemState::Unknown(item.state.0);

    Ok((
        item.id,
        ItemData { kind, frame, owner },
        StateSnapshot {
            frame,
            state,
            owner,
        },
    ))
}

/// Update TurnipLog when encountering new turnips.
fn log_peach_items(log: &mut ItemLog, frame: i32, items: Vec<Item>) {
    for item in items {
        if let Ok((id, data, state)) = parse_item(frame, &item) {
            // create an entry if we haven't seen the turnip before
            let entry = log.entry(id).or_insert(ItemHistory {
                data,
                history: vec![state],
            });

            // update history if state has changed
            if let Some(last_state) = entry.history.last() {
                if last_state.state != state.state {
                    entry.history.push(state);
                }
            }
        }
    }
}

/// Search frames for Peach's turnip pulls.
///
/// Only supports 2-player games.
fn find_turnips(frames: Vec<Frame<2>>) -> ItemLog {
    let mut log: ItemLog = HashMap::new();

    for frame in frames {
        if let Some(items) = frame.items {
            log_peach_items(&mut log, frame.index, items);
        }
    }

    log
}

/// Parse replay file, returning an ItemLog if successful.
async fn read_replay(pool: &sqlx::SqlitePool, path: PathBuf) -> App<ItemLog> {
    let mut conn = pool.acquire().await.expect("failed to acquire connection");

    let f = fs::File::open(&path).map_err(TurnipError::OpenFailed)?;
    let mut buf = io::BufReader::new(f);

    let game = peppi::game(&mut buf, None, None).map_err(TurnipError::ParseFailed)?;

    // 2 player games only
    let frames = match game.frames {
        Frames::P2(f) => f,
        _ => panic!("wrong number of players"),
    };

    let log: ItemLog = find_turnips(frames);
    dbg!(&log);

    // TODO: move to db function / task
    let players = game.metadata.players.expect("no players");
    let start_time = game
        .metadata
        .date
        .expect("failed to get start time")
        .timestamp()
        .to_string();

    let p1: &Player = players
        .iter()
        .filter(|p| matches!(p.port, Port::P1))
        .collect::<Vec<&Player>>()
        .pop()
        .expect("could not find P1");
    let p2: &Player = players
        .iter()
        .filter(|p| matches!(p.port, Port::P2))
        .collect::<Vec<&Player>>()
        .pop()
        .expect("could not find P2");

    let p1_netplay = p1.netplay.as_ref().expect("could not get p1 netplay data");
    let p1_name = &p1_netplay.name;
    let p1_code = &p1_netplay.code;

    let p2_netplay = p2.netplay.as_ref().expect("could not get p2 netplay data");
    let p2_code = &p2_netplay.code;
    let p2_name = &p2_netplay.name;

    let filename = path.to_str().expect("failed to get filename");
    let update = sqlx::query!(
        "INSERT INTO games (filename, start_time, p1_name, p1_code, p2_name, p2_code) VALUES (?, ?, ?, ?, ?, ?)",
        filename, start_time, p1_name, p1_code, p2_name, p2_code
    )
    .execute(&mut conn)
    .await
    .expect("failed to run query");

    dbg!(update);

    Ok(log)
}

/// For each argument, recurse through directories to find replays.
#[tokio::main]
async fn main() {
    // run database migrations
    let pool = SqlitePoolOptions::new()
        .connect(DATABASE_URL)
        .await
        .expect("failed");
    MIGRATOR.run(&pool).await.expect("failed");

    // TODO: instead of acquiring pool for each file,
    // use mpsc queue + message passing w/db worker for updates

    // read replay data
    let directories = env::args().skip(1).collect::<Vec<String>>();
    for path in directories {
        let replays = WalkDir::new(path)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_file())
            .map(|e| read_replay(&pool, e.into_path()))
            .collect::<Vec<_>>();

        let results = futures::future::join_all(replays).await;
        dbg!(results);
    }
}
