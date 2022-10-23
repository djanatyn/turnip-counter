CREATE TABLE IF NOT EXISTS games (
    id INTEGER PRIMARY KEY NOT NULL,
    filename TEXT NOT NULL,
    start_time INTEGER,
    p1_name TEXT NOT NULL,
    p1_code TEXT NOT NULL,
    p2_name TEXT NOT NULL,
    p2_code TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS item (
    id INTEGER PRIMARY KEY NOT NULL,
    game_id INTEGER NOT NULL,
    FOREIGN KEY (game_id) REFERENCES game (id)
);
