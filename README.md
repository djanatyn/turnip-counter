# turnip-counter

walk through slippi replay directory, [reading slp files](https://github.com/hohav/peppi), and record the items that peach pulls into a sqlite database

## usage

```
❯ DATABASE_URL=sqlite://turnips.db cargo sqlx database create
❯ DATABASE_URL=sqlite://turnips.db cargo sqlx migrate run --source db/migrations
Applied 0/migrate init (3.274057ms)
❯ DATABASE_URL=sqlite://turnips.db cargo run -- ~/Slippi/2022-10/
```

```
    Finished dev [unoptimized + debuginfo] target(s) in 0.08s
     Running `target/debug/turnip-counter /home/djanatyn/Slippi/2022-10/`
[src/main.rs:258] &log = {
    112: ItemHistory {
        data: ItemData {
            kind: MrSaturn,
            frame: 11630,
            owner: P1,
        },
        history: [
            StateSnapshot {
                frame: 11630,
                state: Unknown(
                    4,
                ),
                owner: P1,
            },
            StateSnapshot {
                frame: 11681,
                state: Unknown(
                    5,
                ),
                owner: P1,
            },
            StateSnapshot {
                frame: 11879,
                state: Unknown(
                    4,
                ),
                owner: P2,
            },
        ],
    },
    0: ItemHistory {
        data: ItemData {
            kind: NormalTurnip,
            frame: -35,
            owner: P1,
        },
        history: [
            StateSnapshot {
                frame: -35,
                state: Unknown(
                    0,
                ),
                owner: P1,
            },
            StateSnapshot {
                frame: 107,
                state: Unknown(
                    2,
                ),
                owner: P1,
            },
            StateSnapshot {
                frame: 112,
                state: Unknown(
                    1,
                ),
                owner: P1,
            },
        ],
    },
...
```
