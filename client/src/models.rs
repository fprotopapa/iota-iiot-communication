use crate::schema::{
    channels, config, identification, identities, sensor_data, sensor_types, sensors, streams,
    things,
};
use diesel::{Insertable, Queryable};
// Database Models
#[derive(Queryable, Debug)]
pub struct Thing {
    pub id: i32,
    pub thing_key: String,
}

#[derive(Insertable)]
#[table_name = "things"]
pub struct NewThing<'a> {
    pub thing_key: &'a str,
}

#[derive(Queryable, Debug)]
pub struct Channel {
    pub id: i32,
    pub thing_id: i32,
    pub channel_key: String,
}

#[derive(Insertable)]
#[table_name = "channels"]
pub struct NewChannel<'a> {
    pub thing_id: i32,
    pub channel_key: &'a str,
}

#[derive(Queryable, Debug)]
pub struct Identification {
    pub id: i32,
    pub thing_id: i32,
    pub did: String,
    pub vc: Option<String>,
}

#[derive(Insertable)]
#[table_name = "identification"]
pub struct NewIdentification<'a> {
    pub thing_id: i32,
    pub did: &'a str,
    pub vc: &'a str,
}

#[derive(Queryable, Debug)]
pub struct Stream {
    pub id: i32,
    pub channel_id: i32,
    pub ann_link: String,
    pub sub_link: Option<String>,
    pub key_link: Option<String>,
    pub msg_link: Option<String>,
    pub num_subs: Option<i32>,
}

#[derive(Insertable)]
#[table_name = "streams"]
pub struct NewStream<'a> {
    pub channel_id: i32,
    pub ann_link: &'a str,
    pub sub_link: &'a str,
    pub key_link: &'a str,
    pub msg_link: &'a str,
    pub num_subs: i32,
}

#[derive(Queryable, Debug)]
pub struct Identity {
    pub id: i32,
    pub did: String,
    pub verified: Option<bool>,
    pub unverifiable: Option<bool>,
}

#[derive(Insertable)]
#[table_name = "identities"]
pub struct NewIdentity<'a> {
    pub did: &'a str,
    pub verified: bool,
    pub unverifiable: bool,
}

#[derive(Queryable, Debug)]
pub struct Config {
    pub thing_id: i32,
    pub ip: Option<String>,
    pub pk_timestamp: Option<i64>,
}

#[derive(Insertable)]
#[table_name = "config"]
pub struct NewConfiguration<'a> {
    pub ip: &'a str,
    pub pk_timestamp: i64,
}

#[derive(Queryable, Debug)]
pub struct SensorType {
    pub id: i32,
    pub description: String,
    pub unit: Option<String>,
}

#[derive(Insertable)]
#[table_name = "sensor_types"]
pub struct NewSensorType<'a> {
    pub description: &'a str,
    pub unit: &'a str,
}

#[derive(Queryable, Debug)]
pub struct Sensor {
    pub id: i32,
    pub channel_id: i32,
    pub sensor_types_id: i32,
    pub sensor_id: String,
    pub sensor_name: Option<String>,
}

#[derive(Insertable)]
#[table_name = "sensors"]
pub struct NewSensor<'a> {
    pub channel_id: i32,
    pub sensor_types_id: i32,
    pub sensor_id: &'a str,
    pub sensor_name: &'a str,
}

#[derive(Queryable, Debug)]
pub struct SensorData {
    pub id: i32,
    pub sensor_id: i32,
    pub sensor_value: String,
    pub sensor_time: i64,
    pub mqtt: Option<bool>,
    pub iota: Option<bool>,
    pub verified: Option<bool>,
}

#[derive(Insertable)]
#[table_name = "sensor_data"]
pub struct NewSensorData<'a> {
    pub sensor_id: i32,
    pub sensor_value: &'a str,
    pub sensor_time: i64,
    pub mqtt: bool,
    pub iota: bool,
    pub verified: bool,
}
