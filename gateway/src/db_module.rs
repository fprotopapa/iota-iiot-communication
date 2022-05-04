#![allow(dead_code)]
use crate::models;
use crate::schema;

use diesel::prelude::*;
use dotenv::dotenv;
use std::env;

use schema::{
    channels, config, identification, identities, sensor_data, sensor_types, sensors, streams,
    things,
};

/// Connect to Database
pub fn establish_connection() -> SqliteConnection {
    dotenv().ok();

    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    info!("DB Connection Established");
    SqliteConnection::establish(&database_url)
        .expect(&format!("Error connecting to {}", database_url))
}
/// Select Sensor
pub fn select_sensor(
    conn: &SqliteConnection,
    sensor_identifier: i32,
) -> Result<models::Sensor, i32> {
    use self::sensors::dsl::*;
    let entry = match sensors
        .filter(id.eq(sensor_identifier))
        .limit(1)
        .get_result::<models::Sensor>(conn)
    {
        Ok(r) => r,
        Err(e) => {
            error!("{}", e);
            return Err(-1);
        }
    };
    Ok(entry)
}
/// Select Sensor
pub fn select_sensor_by_name(
    conn: &SqliteConnection,
    sensor_identifier: &str,
) -> Result<models::Sensor, i32> {
    use self::sensors::dsl::*;
    let entry = match sensors
        .filter(sensor_id.eq(sensor_identifier))
        .limit(1)
        .get_result::<models::Sensor>(conn)
    {
        Ok(r) => r,
        Err(e) => {
            error!("{}", e);
            return Err(-1);
        }
    };
    Ok(entry)
}
/// Select Sensor Entry
pub fn select_sensor_entry(
    conn: &SqliteConnection,
    query: &str,
    is_true: bool,
    timestamp: i64,
) -> Result<Vec<models::SensorData>, i32> {
    use self::sensor_data::dsl::*;
    let query = match query {
        "mqtt" => sensor_data
            .filter(mqtt.eq(is_true))
            .limit(20)
            .get_results::<models::SensorData>(conn),
        "iota" => sensor_data
            .filter(iota.eq(is_true))
            .limit(20)
            .get_results::<models::SensorData>(conn),
        "verified" => sensor_data
            .filter(verified.eq(is_true))
            .limit(20)
            .get_results::<models::SensorData>(conn),
        "timestamp" => sensor_data
            .filter(sensor_time.gt(timestamp)) // greater then
            .limit(20)
            .get_results::<models::SensorData>(conn),
        e => {
            error!("{}", e);
            return Err(-1);
        }
    };
    let results = match query {
        Ok(r) => r,
        Err(e) => {
            error!("{}", e);
            return Err(-1);
        }
    };
    Ok(results)
}
/// Select Thing
pub fn select_thing(conn: &SqliteConnection, th_key: &str) -> Result<models::Thing, i32> {
    use self::things::dsl::*;
    // Scope DB Modell because of same protobuf name
    let entry = match things
        .filter(thing_key.eq(th_key))
        .limit(1)
        .get_result::<models::Thing>(conn)
    {
        Ok(r) => r,
        Err(e) => {
            error!("{}", e);
            return Err(-1);
        }
    };
    Ok(entry)
}
/// Select Channel
pub fn select_channel(conn: &SqliteConnection, ch_key: &str) -> Result<models::Channel, i32> {
    use self::channels::dsl::*;
    // Scope DB Modell because of same protobuf name
    let entry = match channels
        .filter(channel_key.eq(ch_key))
        .limit(1)
        .get_result::<models::Channel>(conn)
    {
        Ok(r) => r,
        Err(e) => {
            error!("{}", e);
            return Err(-1);
        }
    };
    Ok(entry)
}
/// Table Things CRUD
/// Create Thing
pub fn create_thing<'a>(conn: &SqliteConnection, thing_key: &'a str) -> Result<usize, i32> {
    let new_entry = models::NewThing {
        thing_key: thing_key,
    };

    let entry = match diesel::insert_into(things::table)
        .values(&new_entry)
        .execute(conn)
    {
        Ok(r) => r,
        Err(e) => {
            error!("{}", e);
            return Err(-1);
        }
    };
    Ok(entry)
}
/// Table Channels CRUD
/// Create Channel
pub fn create_channel<'a>(
    conn: &SqliteConnection,
    thing_id: i32,
    channel_key: &'a str,
) -> Result<usize, i32> {
    let new_entry = models::NewChannel {
        thing_id: thing_id,
        channel_key: channel_key,
    };

    let entry = match diesel::insert_into(channels::table)
        .values(&new_entry)
        .execute(conn)
    {
        Ok(r) => r,
        Err(e) => {
            error!("{}", e);
            return Err(-1);
        }
    };
    Ok(entry)
}
/// Update Identity
pub fn update_identity(
    conn: &SqliteConnection,
    digital_id: &str,
    verification: bool,
) -> Result<i32, i32> {
    use self::identities::dsl::*;
    match diesel::update(identities)
        .filter(did.eq(digital_id))
        .set(verified.eq(verification))
        .execute(conn)
    {
        Ok(r) => {
            info!("Affected Rows: {}", r);
            return Ok(r as i32);
        }
        Err(e) => {
            error!("{}", e);
            return Err(-1);
        }
    };
}
/// Update Identity Make Unverifiable
pub fn update_identity_to_unverifiable(
    conn: &SqliteConnection,
    digital_id: &str,
    is_unverifiable: bool,
) -> Result<i32, i32> {
    use self::identities::dsl::*;
    match diesel::update(identities)
        .filter(did.eq(digital_id))
        .set(unverifiable.eq(is_unverifiable))
        .execute(conn)
    {
        Ok(r) => {
            info!("Affected Rows: {}", r);
            return Ok(r as i32);
        }
        Err(e) => {
            error!("{}", e);
            return Err(-1);
        }
    };
}
/// Update Identity Make Unverifiable
pub fn update_identity_to_subscribed(
    conn: &SqliteConnection,
    digital_id: &str,
    is_subscribed: bool,
) -> Result<i32, i32> {
    use self::identities::dsl::*;
    match diesel::update(identities)
        .filter(did.eq(digital_id))
        .set(subscribed.eq(is_subscribed))
        .execute(conn)
    {
        Ok(r) => {
            info!("Affected Rows: {}", r);
            return Ok(r as i32);
        }
        Err(e) => {
            error!("{}", e);
            return Err(-1);
        }
    };
}
/// Select Identity
pub fn select_identity(conn: &SqliteConnection, digital_id: &str) -> Result<models::Identity, i32> {
    use self::identities::dsl::*;
    let entry = match identities
        .filter(did.eq(digital_id))
        .limit(1)
        .get_result::<models::Identity>(conn)
    {
        Ok(r) => r,
        Err(e) => {
            error!("{}", e);
            return Err(-1);
        }
    };
    Ok(entry)
}
/// Select Sensor Entry
pub fn select_identities(
    conn: &SqliteConnection,
    is_verified: bool,
) -> Result<Vec<models::Identity>, i32> {
    use self::identities::dsl::*;
    let results = match identities
        .filter(verified.eq(is_verified))
        .limit(10)
        .get_results::<models::Identity>(conn)
    {
        Ok(r) => r,
        Err(e) => {
            error!("{}", e);
            return Err(-1);
        }
    };
    Ok(results)
}
/// Select Identification
pub fn select_identification(
    conn: &SqliteConnection,
    thing_identifier: i32,
) -> Result<models::Identification, i32> {
    use self::identification::dsl::*;
    // Scope DB Modell because of same protobuf name
    let entry = match identification
        .filter(thing_id.eq(thing_identifier))
        .limit(1)
        .get_result::<models::Identification>(conn)
    {
        Ok(r) => r,
        Err(e) => {
            error!("{}", e);
            return Err(-1);
        }
    };
    Ok(entry)
}
/// Table Identification CRUD
/// Create Identification
pub fn create_identification<'a>(
    conn: &SqliteConnection,
    thing_id: i32,
    did: &'a str,
    vc: &str,
) -> Result<usize, i32> {
    let new_entry = models::NewIdentification {
        thing_id: thing_id,
        did: did,
        vc: vc,
    };

    let entry = match diesel::insert_into(identification::table)
        .values(&new_entry)
        .execute(conn)
    {
        Ok(r) => r,
        Err(e) => {
            error!("{}", e);
            return Err(-1);
        }
    };
    Ok(entry)
}
/// Select Stream
pub fn select_stream(
    conn: &SqliteConnection,
    channel_identifier: i32,
) -> Result<models::Stream, i32> {
    use self::streams::dsl::*;
    let entry = match streams
        .filter(channel_id.eq(channel_identifier))
        .limit(1)
        .get_result::<models::Stream>(conn)
    {
        Ok(r) => r,
        Err(e) => {
            error!("{}", e);
            return Err(-1);
        }
    };
    Ok(entry)
}
/// Table Streams CRUD
/// Struct to Hold Streams Table Entry Data
pub struct StreamsEntry {
    pub channel_id: i32,
    pub ann_link: String,
    pub sub_link: String,
    pub key_link: String,
    pub msg_link: String,
}
/// Create Stream
pub fn create_stream<'a>(conn: &SqliteConnection, entry: StreamsEntry) -> Result<usize, i32> {
    let new_entry = models::NewStream {
        channel_id: entry.channel_id,
        ann_link: &entry.ann_link,
        sub_link: &entry.sub_link,
        key_link: &entry.key_link,
        msg_link: &entry.msg_link,
        num_subs: 0,
    };

    let entry = match diesel::insert_into(streams::table)
        .values(&new_entry)
        .execute(conn)
    {
        Ok(r) => r,
        Err(e) => {
            error!("{}", e);
            return Err(-1);
        }
    };
    Ok(entry)
}
/// Update Configuration
pub fn update_configuration(
    conn: &SqliteConnection,
    thing_identifier: i32,
    query: &str,
    ext_ip: &str,
    timestamp: i64,
) -> Result<i32, i32> {
    use self::config::dsl::*;
    let query = match query {
        "ip" => diesel::update(config.filter(thing_id.eq(thing_identifier)))
            .set(ip.eq(ext_ip))
            .execute(conn),
        "pk_timestamp" => diesel::update(config.filter(thing_id.eq(thing_identifier)))
            .set(pk_timestamp.eq(timestamp))
            .execute(conn),
        _ => return Err(-1),
    };
    match query {
        Ok(r) => {
            info!("Affected Rows: {}", r);
            return Ok(r as i32);
        }
        Err(e) => {
            error!("{}", e);
            return Err(-1);
        }
    };
}
/// Table Identities CRUD
/// Create Identity
pub fn create_identity<'a>(
    conn: &SqliteConnection,
    did: &'a str,
    verified: bool,
) -> Result<usize, i32> {
    let new_entry = models::NewIdentity {
        did: did,
        verified: verified,
        unverifiable: false,
        subscribed: false,
    };

    let entry = match diesel::insert_into(identities::table)
        .values(&new_entry)
        .execute(conn)
    {
        Ok(r) => r,
        Err(e) => {
            error!("{}", e);
            return Err(-1);
        }
    };
    Ok(entry)
}
/// Table Config CRUD
/// Create Config
pub fn create_configuration(
    conn: &SqliteConnection,
    ext_ip: &str,
    pk_timestamp: i64,
) -> Result<usize, i32> {
    let new_entry = models::NewConfiguration {
        ip: ext_ip,
        pk_timestamp: pk_timestamp,
    };

    let entry = match diesel::insert_into(config::table)
        .values(&new_entry)
        .execute(conn)
    {
        Ok(r) => r,
        Err(e) => {
            error!("{}", e);
            return Err(-1);
        }
    };
    Ok(entry)
}
/// Update Stream
pub fn update_stream(
    conn: &SqliteConnection,
    channel_identifier: i32,
    query: &str,
    link: &str,
    num_subscribers: i32,
) -> Result<i32, i32> {
    use self::streams::dsl::*;
    let query = match query {
        "announcement" => diesel::update(streams.filter(channel_id.eq(channel_identifier)))
            .set(ann_link.eq(link))
            .execute(conn),
        "subscription" => diesel::update(streams.filter(channel_id.eq(channel_identifier)))
            .set(sub_link.eq(link))
            .execute(conn),
        "keyload" => diesel::update(streams.filter(channel_id.eq(channel_identifier)))
            .set(key_link.eq(link))
            .execute(conn),
        "msg_link" => diesel::update(streams.filter(channel_id.eq(channel_identifier)))
            .set(msg_link.eq(link))
            .execute(conn),
        "num_subs" => diesel::update(streams.filter(channel_id.eq(channel_identifier)))
            .set(num_subs.eq(num_subscribers))
            .execute(conn),
        _ => {
            error!("{}", query);
            return Err(-1);
        }
    };
    match query {
        Ok(r) => {
            info!("Affected Rows: {}", r);
            return Ok(r as i32);
        }
        Err(e) => {
            error!("{}", e);
            return Err(-1);
        }
    };
}
/// Update Sensor Entry
pub fn update_sensor_entry(
    conn: &SqliteConnection,
    identifier: i32,
    query: &str,
    is_true: bool,
) -> Result<i32, i32> {
    use self::sensor_data::dsl::*;
    let query = match query {
        "mqtt" => diesel::update(sensor_data.filter(id.eq(identifier)))
            .set(mqtt.eq(is_true))
            .execute(conn),
        "iota" => diesel::update(sensor_data.filter(id.eq(identifier)))
            .set(iota.eq(is_true))
            .execute(conn),
        "verified" => diesel::update(sensor_data.filter(id.eq(identifier)))
            .set(verified.eq(is_true))
            .execute(conn),
        e => {
            error!("{}", e);
            return Err(-1);
        }
    };
    match query {
        Ok(r) => {
            info!("Affected Rows: {}", r);
            return Ok(r as i32);
        }
        Err(e) => {
            error!("{}", e);
            return Err(-1);
        }
    };
}
/// Select Sensor Type
pub fn select_sensor_type_by_desc(
    conn: &SqliteConnection,
    descr: &str,
) -> Result<models::SensorType, i32> {
    use self::sensor_types::dsl::*;
    // Scope DB Modell because of same protobuf name
    let entry = match sensor_types
        .filter(description.eq(descr))
        .limit(1)
        .get_result::<models::SensorType>(conn)
    {
        Ok(r) => r,
        Err(e) => {
            error!("{}", e);
            return Err(-1);
        }
    };
    Ok(entry)
}
/// Select Sensor Type
pub fn select_sensor_type_by_id(
    conn: &SqliteConnection,
    identifier: i32,
) -> Result<models::SensorType, i32> {
    use self::sensor_types::dsl::*;
    // Scope DB Modell because of same protobuf name
    let entry = match sensor_types
        .filter(id.eq(identifier))
        .limit(1)
        .get_result::<models::SensorType>(conn)
    {
        Ok(r) => r,
        Err(e) => {
            error!("{}", e);
            return Err(-1);
        }
    };
    Ok(entry)
}
/// Table Sensor Types CRUD
/// Create Sensor Type
pub fn create_sensor_type<'a>(
    conn: &SqliteConnection,
    description: &'a str,
    unit: &'a str,
) -> Result<usize, i32> {
    let new_entry = models::NewSensorType {
        description: description,
        unit: unit,
    };

    let entry = match diesel::insert_into(sensor_types::table)
        .values(&new_entry)
        .execute(conn)
    {
        Ok(r) => r,
        Err(e) => {
            error!("{}", e);
            return Err(-1);
        }
    };
    Ok(entry)
}
/// Table Sensors CRUD
/// Struct Sensor Entry
pub struct SensorEntry {
    pub channel_id: i32,
    pub sensor_types_id: i32,
    pub sensor_id: String,
    pub sensor_name: String,
}
/// Create Sensor
pub fn create_sensor<'a>(conn: &SqliteConnection, entry: SensorEntry) -> Result<usize, i32> {
    let new_entry = models::NewSensor {
        channel_id: entry.channel_id,
        sensor_types_id: entry.sensor_types_id,
        sensor_id: &entry.sensor_id,
        sensor_name: &entry.sensor_name,
    };

    let entry = match diesel::insert_into(sensors::table)
        .values(&new_entry)
        .execute(conn)
    {
        Ok(r) => r,
        Err(e) => {
            error!("{}", e);
            return Err(-1);
        }
    };
    Ok(entry)
}

/// Table Sensor Data CRUD
/// Struct Sensor Data Entry
pub struct SensorDataEntry {
    pub sensor_id: i32,
    pub sensor_value: String,
    pub sensor_time: i64,
    pub mqtt: bool,
    pub iota: bool,
    pub verified: bool,
}
/// Create Sensor Data
pub fn create_sensor_data<'a>(
    conn: &SqliteConnection,
    entry: SensorDataEntry,
) -> Result<usize, i32> {
    let new_entry = models::NewSensorData {
        sensor_id: entry.sensor_id,
        sensor_value: &entry.sensor_value,
        sensor_time: entry.sensor_time,
        mqtt: entry.mqtt,
        iota: entry.iota,
        verified: entry.verified,
    };

    let entry = match diesel::insert_into(sensor_data::table)
        .values(&new_entry)
        .execute(conn)
    {
        Ok(r) => r,
        Err(e) => {
            error!("{}", e);
            return Err(-1);
        }
    };
    Ok(entry)
}
