# turnip-counter

walk through slippi replay directory, [reading slp files](https://github.com/hohav/peppi), and record the items that peach pulls into a sqlite database

![](https://ssb.wiki.gallery/images/thumb/6/63/Peach_Down_Special_Pull_Hitbox_Melee.gif/300px-Peach_Down_Special_Pull_Hitbox_Melee.gif)

## implementation

The code looks specifically for [items initially owned by me](https://github.com/djanatyn/turnip-counter/blob/master/src/main.rs#L165), so the good luck of other Peach players isn't counted:
```rust
/// Tokio task to send Peach item records to database worker.
async fn record_items(tx: Sender<DBCommand>, items: ItemLog, game_id: i64, me: Port) -> App<()> {
    for (item_id, history) in items {
        // only record my turnips
        if history.data.owner != me {
            continue;
        }
        // ...
    }
}
```

I'm using the [`peppi`](https://lib.rs/crates/peppi) Rust library to parse replays. I used `tokio` for async, `thiserror` for error handling, `sqlx` for typed queries + db migrations, and `walkdir` for recursively walking through replay directories.

## background

When Peach pulls items in melee, the [items pulled are random](https://www.ssbwiki.com/Peach_(SSBM)/Down_special), and we have expected values based on our [understanding of Melee's RNG](https://www.reddit.com/r/SSBM/comments/71gn1d/the_basics_of_rng_in_melee/):
```
| Type                                   | Damage | Probability         |
|----------------------------------------|--------|---------------------|
| Normal                                 | 6-10%  | 4445/7424 ≈ 59.873% |
| Carrot Eyes                            | 6-10%  | 762/7424 ≈ 10.264%  |
| Line Eyes                              | 6-10%  | 635/7424 ≈ 8.553%   |
| Circle Eyes                            | 6-10%  | 381/7424 ≈ 5.132%   |
| Eyebrow Eyes                           | 6-10%  | 381/7424 ≈ 5.132%   |
| Wink                                   | 10-14% | 508/7424 ≈ 6.843%   |
| Dot Eyes                               | 16-20% | 127/7424 ≈ 1.711%   |
| Stitch Face                            | 34-38% | 127/7424 ≈ 1.711%   |
| [Mr. Saturn](/Mr._Saturn "Mr. Saturn") | —      | 3/768 ≈ 0.391%      |
| [Bob-omb](/Bob-omb "Bob-omb")          | —      | 2/768 ≈ 0.260%      |
| [Beam Sword](/Beam_Sword "Beam Sword") | —      | 1/768 ≈ 0.130%      |
```

## example usage

With that information, I can compare my **actual frequency** of items pulled vs the **predicted frequency** of items pulled. It's very close! I have 418 replays from this month (October 2022) to analyze:
```
❯ DATABASE_URL=sqlite://turnips.db cargo sqlx database create
❯ DATABASE_URL=sqlite://turnips.db cargo sqlx migrate run --source db/migrations
Applied 0/migrate init (3.274057ms)
❯ DATABASE_URL=sqlite://turnips.db cargo run -- ~/Slippi/2022-10/
...
```
```sql
sqlite> SELECT COUNT(*) FROM items;
3505
sqlite> SELECT COUNT(*) FROM games;
418
sqlite> SELECT kind, CAST(COUNT(*) AS REAL) / (SELECT COUNT(*) FROM items) FROM items GROUP BY kind;
Beamsword      0.00114122681883024 -- ~0.11% actual vs 0.13% predicted
Bobomb         0.00228245363766049 -- ~0.23% actual vs 0.26% predicted
DotEyesTurnip  0.0145506419400856  -- ~1.46% actual vs 1.71% predicted
MrSaturn       0.00485021398002853 -- ~0.46% actual vs 0.39% predicted
NormalTurnip   0.888445078459344   -- ~88.84% actual vs 88.95% predicted (59.873 + 10.264 + 8.553 + 5.132 + 5.132)
StitchTurnip   0.0182596291012839  -- ~1.82% actual vs 1.71% predicted
WinkyTurnip    0.0704707560627675  -- ~7.04% actual vs 6.84% predicted
```
