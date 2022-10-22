#![allow(dead_code)]

//! TODO: how are you outputting this data?
//! - csv, sqlite?
//! - JSON for d3 or plotly

//! TODO: search slippi directory
//! TODO: get timetsamp of replay
//! TODO: calculate timestamp of each turnip pull, given TurnipLog
//! TODO: output information as JSON
//! TODO: d3 visualization for time series
//! - <https://observablehq.com/@d3/stacked-bar-chart>
//! - <https://plotly.com/javascript/histograms/#colored-and-styled-histograms>

use peppi::model::enums::item::Type;
use peppi::model::frame::Frame;
use peppi::model::game::Frames;
use peppi::model::item::Item;
use peppi::model::primitives::Port;
use std::collections::HashMap;
use std::{env, fs, io};

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
fn parse_item(frame: i32, item: &Item) -> Option<(u32, ItemData, StateSnapshot)> {
    // TODO: parse all peach items

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
                _ => panic!("unknown state"),
            }
        }
        _ => return None,
    };

    let owner: Port = item.owner.expect("no owner").expect("no player");

    // we don't know what item states are yet
    let state = ItemState::Unknown(item.state.0);

    Some((
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
        if let Some((id, data, state)) = parse_item(frame, &item) {
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
        let log: ItemLog = find_turnips(frames);
        println!("{log:#?}");
    }
}
