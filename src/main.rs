#![allow(dead_code)]

use peppi::model::enums::item::Type;
use peppi::model::frame::Frame;
use peppi::model::game::Frames;
use peppi::model::item::Item;
use peppi::model::primitives::Port;
use std::collections::HashMap;
use std::{env, fs, io};

/// Possible turnip faces.
/// Taken from second byte of misc field.
#[derive(Debug)]
enum TurnipFace {
    /// `(0..4).contains(misc[1])`
    Normal,
    /// `misc[1] == 5`
    Winky,
    /// `misc[1] == 6`
    DotEyes,
    /// `misc[1] == 7`
    Stitch,
}

#[derive(Debug)]
struct TurnipData {
    /// The face of the turnip.
    face: TurnipFace,

    /// The first frame the turnip was seen.
    frame: i32,

    /// The initial owner of the turnip.
    owner: Port,
}

/// Index pulled turnips by item ID.
type TurnipLog = HashMap<u32, TurnipData>;

/// Check to see if the item is a turnip.
fn parse_turnip(frame: i32, item: &Item) -> Option<(u32, TurnipData)> {
    if item.r#type != Type::PEACH_TURNIP {
        return None;
    };

    let owner: Port = item.owner.expect("no owner").expect("no player");

    // turnip face data is in second byte of misc field
    let face_byte = item.misc.expect("no misc data")[1];
    let face: TurnipFace = match face_byte {
        0 => TurnipFace::Normal,
        1 => TurnipFace::Normal,
        2 => TurnipFace::Normal,
        3 => TurnipFace::Normal,
        4 => TurnipFace::Normal,
        5 => TurnipFace::Winky,
        6 => TurnipFace::DotEyes,
        7 => TurnipFace::Stitch,
        _ => panic!("unknown state"),
    };

    Some((item.id, TurnipData { face, frame, owner }))
}

/// Update TurnipLog when encountering new turnips.
fn log_peach_items(log: &mut TurnipLog, frame: i32, items: Vec<Item>) {
    for item in items {
        if let Some((id, data)) = parse_turnip(frame, &item) {
            // create an entry if we haven't seen the turnip before
            (*log).entry(id).or_insert(data);
        }
    }
}

/// Search frames for Peach's turnip pulls.
///
/// Only supports 2-player games.
fn find_turnips(frames: Vec<Frame<2>>) -> TurnipLog {
    let mut log: TurnipLog = HashMap::new();

    for frame in frames {
        if let Some(items) = frame.items {
            log_peach_items(&mut log, frame.index, items);
        }
    }

    log
}

/// For each argument, count turnips.
fn main() {
    let files = env::args().skip(1).collect::<Vec<String>>();

    for filename in files {
        let f = fs::File::open(filename).expect("failed to open");
        let mut buf = io::BufReader::new(f);

        let game = peppi::game(&mut buf, None, None).expect("failed to parse");

        // 2 player games only
        let frames = match game.frames {
            Frames::P2(f) => f,
            _ => panic!("wrong number of players"),
        };

        // print turnip log
        let log: TurnipLog = find_turnips(frames);
        println!("{log:#?}");
    }
}
