use peppi::model::frame::Frame;
use peppi::model::game::Frames;
use peppi::model::item::Item;
use std::{env, fs, io};

const BOBOMB_ID: u32 = 6;
const MR_SATURN_ID: u32 = 7;
const BEAM_SWORD_ID: u32 = 12;
const TURNIP_ID: u32 = 99;

const PULLABLES: [u32; 4] = [BOBOMB_ID, MR_SATURN_ID, BEAM_SWORD_ID, TURNIP_ID];

/// Filter items for Peach's Vegetable pulls.
fn peach_items(items: Vec<Item>) -> Vec<Item> {
    items
        .iter()
        .filter(|item| PULLABLES.contains(&item.id))
        .cloned()
        .collect()
}

/// Search frames for Peach's items.
fn find_items(frames: Vec<Frame<2>>) {
    for frame in frames {
        if let Some(items) = frame.items {
            let pulls = peach_items(items);
            // show items peach pulled
            if !pulls.is_empty() {
                dbg!(pulls);
            }
        }
    }
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
