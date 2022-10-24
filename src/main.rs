#![allow(dead_code)]

//! TODO: session table, session start time, start -> {success, error}
//! TODO: d3 visualization for time series
//! - <https://observablehq.com/@d3/stacked-bar-chart>
//! - <https://plotly.com/javascript/histograms/#colored-and-styled-histograms>

use futures::future::join_all;
use peppi::model::enums::item::Type;
use peppi::model::game::{Frames, Game};
use peppi::model::{frame::Frame, item::Item, metadata::Player, primitives::Port};
use sqlx::migrate::Migrator;
use sqlx::sqlite::SqlitePoolOptions;
use std::collections::HashMap;
use std::path::PathBuf;
use std::{env, fs, io};
use thiserror::Error;
use tokio::sync::mpsc::{self, Receiver, Sender};
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

#[derive(Debug, Clone)]
struct GameMetadata {
    filename: String,
    start_time: String,
    p1_name: String,
    p1_code: String,
    p2_name: String,
    p2_code: String,
    my_port: Port,
}

#[derive(Debug, Clone)]
enum DBCommand {
    Item {
        game_id: String,
        item_id: String,
        frame: String,
        kind: String,
    },
}

/// Index pulled turnips by item ID.
type ItemLog = HashMap<u32, ItemHistory>;

/// Parse replay file with peppi.
fn parse_replay(path: &PathBuf) -> App<Game> {
    let f = fs::File::open(&path).map_err(TurnipError::OpenFailed)?;
    let mut buf = io::BufReader::new(f);

    peppi::game(&mut buf, None, None).map_err(TurnipError::ParseFailed)
}

/// Extract metadata to record for parsed game.
fn game_metadata(path: PathBuf, game: &Game) -> App<GameMetadata> {
    let players = game.metadata.players.as_ref().expect("no players");
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
    let p1_name = p1_netplay.name.to_string();
    let p1_code = p1_netplay.code.to_string();

    let p2_netplay = p2.netplay.as_ref().expect("could not get p2 netplay data");
    let p2_code = p2_netplay.code.to_owned();
    let p2_name = p2_netplay.code.to_owned();

    let filename = path.to_str().expect("failed to get filename").to_string();

    let my_port: Port = [(&p1_code, &p1), (&p2_code, &p2)]
        .iter()
        .filter_map(|(code, player)| (*code == "ACAB#420").then(|| *player))
        .collect::<Vec<&&Player>>()
        .pop()
        .expect("failed to find DJAN!")
        .port;

    Ok(GameMetadata {
        filename,
        start_time,
        p1_name,
        p1_code,
        p2_name,
        p2_code,
        my_port,
    })
}

/// Parse replay file, returning an ItemLog if successful.
fn log_items(game: Game) -> App<ItemLog> {
    // 2 player games only
    let frames = match game.frames {
        Frames::P2(f) => f,
        _ => panic!("wrong number of players"),
    };

    Ok(find_turnips(frames))
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

/// Tokio task to send Peach item records to database worker.
async fn record_items(tx: Sender<DBCommand>, items: ItemLog, game_id: i64, me: Port) -> App<()> {
    for (item_id, history) in items {
        // only record my turnips
        if history.data.owner != me {
            continue;
        }

        let kind = format!("{:?}", history.data.kind);
        tx.send(DBCommand::Item {
            game_id: game_id.to_string(),
            item_id: item_id.to_string(),
            frame: history.data.frame.to_string(),
            kind,
        })
        .await
        .expect("failed to send");
    }

    Ok(())
}

/// Tokio task for database updates.
async fn db_worker(mut rx: Receiver<DBCommand>) -> App<()> {
    let pool = SqlitePoolOptions::new()
        .connect(DATABASE_URL)
        .await
        .expect("failed");
    let mut conn = pool.acquire().await.expect("failed to acquire connection");
    while let Some(cmd) = rx.recv().await {
        match dbg!(cmd) {
            DBCommand::Item {
                game_id,
                item_id,
                frame,
                kind,
            } => {
                sqlx::query!(
                    "INSERT INTO items (game_id, item_id, frame, kind) VALUES (?, ?, ?, ?)",
                    game_id,
                    item_id,
                    frame,
                    kind
                )
                .execute(&mut conn)
                .await
                .expect("failed to record item");
            }
        }
    }
    Ok(())
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

    // open up message queue for db commands
    let (tx, rx) = mpsc::channel::<DBCommand>(32);
    let db_task = tokio::spawn(db_worker(rx));

    // read replay directory, get replay file paths
    let replay_directory = env::args().skip(1).collect::<String>();
    let replays = WalkDir::new(replay_directory)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter_map(|e| (e.file_type().is_file()).then(|| e.into_path()))
        .collect::<Vec<PathBuf>>();

    // tokio tasks for recording items
    let mut item_tasks = Vec::new();

    // process replay data
    for replay in &replays {
        // parse replay with peppi
        let game = match parse_replay(replay) {
            Ok(replay) => replay,
            Err(_) => continue,
        };

        // extract metadata
        let metadata = game_metadata(replay.to_path_buf(), &game).expect("failed to get metadata");

        // create game row in DB, get ID
        let mut conn = pool.acquire().await.expect("failed to acquire conn");
        let update = sqlx::query!(
            "INSERT INTO games (filename, start_time, p1_name, p1_code, p2_name, p2_code) VALUES (?, ?, ?, ?, ?, ?)",
            metadata.filename, metadata.start_time, metadata.p1_name, metadata.p1_code, metadata.p2_name, metadata.p2_code
        )
        .execute(&mut conn)
        .await
        .expect("failed to run query");
        let game_id = update.last_insert_rowid();

        // get all peach items
        let items: ItemLog = log_items(game).expect("failed");

        // write items to db
        item_tasks.push(tokio::spawn(record_items(
            tx.clone(),
            items,
            game_id,
            metadata.my_port,
        )));
    }

    // wait for item log tasks to finish
    join_all(item_tasks).await;

    // wait for db commands to finish
    db_task.await.expect("failed").expect("db task failed");
}
