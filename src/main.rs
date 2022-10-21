use peppi::model::enums::item::Type;
use peppi::model::frame::Frame;
use peppi::model::game::Frames;
use peppi::model::item::Item;
use std::collections::HashMap;
use std::{env, fs, io};

/// Possible turnip faces.
#[derive(Debug)]
enum TurnipFace {
    Normal,  // 0-4
    Winky,   // 5
    DotEyes, // 6
    Stitch,  // 7
}

/// Index pulled turnip faces by item ID.
type TurnipLog = HashMap<u32, TurnipFace>;

/// Check to see if the item is a turnip.
/// If it is, return the item ID + TurnipFace.
fn is_turnip(item: &Item) -> Option<(u32, TurnipFace)> {
    if item.r#type == Type::PEACH_TURNIP {
        let face: TurnipFace = match item.state.0 {
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

        Some((item.id, face))
    } else {
        None
    }
}

fn log_peach_items(log: &mut TurnipLog, items: Vec<Item>) {
    for item in items {
        if let Some((id, face)) = is_turnip(&item) {
            (*log).insert(id, face);
        }
    }
}

/// Search frames for Peach's items.
fn find_items(frames: Vec<Frame<2>>) {
    let mut log: TurnipLog = HashMap::new();

    for frame in frames {
        if let Some(items) = frame.items {
            log_peach_items(&mut log, items);
        }
    }

    dbg!(log);
}

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

        find_items(frames);
    }
}
