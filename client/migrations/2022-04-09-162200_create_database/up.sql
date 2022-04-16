-- Your SQL goes here
CREATE TABLE IF NOT EXISTS things (
    id INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL,
    thing_key TEXT NOT NULL UNIQUE
);

CREATE TABLE IF NOT EXISTS channels (
id INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL,
thing_id INTEGER NOT NULL,
channel_key TEXT NOT NULL UNIQUE,
CONSTRAINT fk_thing FOREIGN KEY(thing_id) REFERENCES things(id)
);

CREATE TABLE IF NOT EXISTS identification (
    id INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL,
    thing_id INTEGER NOT NULL,
    did TEXT NOT NULL UNIQUE,
    vc TEXT,
    FOREIGN KEY (thing_id)
        REFERENCES things (id)
);

CREATE TABLE IF NOT EXISTS streams (
    id INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL,
    channel_id INTEGER NOT NULL,
    ann_link TEXT NOT NULL UNIQUE,
    sub_link TEXT,
    key_link TEXT,
    msg_link TEXT,
    num_subs INTEGER,
    FOREIGN KEY (channel_id)
        REFERENCES channels (id)
);

CREATE TABLE IF NOT EXISTS identities (
    id INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL,
    did TEXT NOT NULL UNIQUE,
    verified BOOLEAN DEFAULT FALSE,
    unverifiable BOOLEAN DEFAULT FALSE
);

CREATE TABLE IF NOT EXISTS config (
    thing_id INTEGER NOT NULL,
    ip TEXT,
    pk_timestamp BIGINT DEFAULT 0,
    PRIMARY KEY (thing_id),
    FOREIGN KEY (thing_id)
        REFERENCES things (id)
);

CREATE TABLE IF NOT EXISTS sensor_types (
    id INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL,
    description TEXT NOT NULL UNIQUE,
    unit TEXT
);

CREATE TABLE IF NOT EXISTS sensors (
    id INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL,
    channel_id INTEGER NOT NULL,
    sensor_types_id INTEGER NOT NULL,
    sensor_id TEXT NOT NULL UNIQUE,
    sensor_name TEXT,
    FOREIGN KEY (channel_id)
        REFERENCES channels (id),
    FOREIGN KEY (sensor_types_id)
        REFERENCES sensor_types (id)
);

CREATE TABLE IF NOT EXISTS sensor_data (
    id INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL,
    sensor_id INTEGER NOT NULL,
    sensor_value TEXT NOT NULL,
    sensor_time BIGINT NOT NULL,
    mqtt BOOLEAN DEFAULT FALSE,
    iota BOOLEAN DEFAULT FALSE,
    verified BOOLEAN DEFAULT FALSE,
    FOREIGN KEY (sensor_id)
        REFERENCES sensors (id)
);