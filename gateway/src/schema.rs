table! {
    channels (id) {
        id -> Integer,
        thing_id -> Integer,
        channel_key -> Text,
    }
}

table! {
    config (thing_id) {
        thing_id -> Integer,
        ip -> Nullable<Text>,
        pk_timestamp -> Nullable<BigInt>,
    }
}

table! {
    identification (id) {
        id -> Integer,
        thing_id -> Integer,
        did -> Text,
        vc -> Nullable<Text>,
    }
}

table! {
    identities (id) {
        id -> Integer,
        did -> Text,
        verified -> Nullable<Bool>,
        unverifiable -> Nullable<Bool>,
        subscribed -> Nullable<Bool>,
    }
}

table! {
    sensor_data (id) {
        id -> Integer,
        sensor_id -> Integer,
        sensor_value -> Text,
        sensor_time -> BigInt,
        mqtt -> Nullable<Bool>,
        iota -> Nullable<Bool>,
        verified -> Nullable<Bool>,
    }
}

table! {
    sensor_types (id) {
        id -> Integer,
        description -> Text,
        unit -> Nullable<Text>,
    }
}

table! {
    sensors (id) {
        id -> Integer,
        channel_id -> Integer,
        sensor_types_id -> Integer,
        sensor_id -> Text,
        sensor_name -> Nullable<Text>,
    }
}

table! {
    streams (id) {
        id -> Integer,
        channel_id -> Integer,
        ann_link -> Text,
        sub_link -> Nullable<Text>,
        key_link -> Nullable<Text>,
        msg_link -> Nullable<Text>,
        num_subs -> Nullable<Integer>,
    }
}

table! {
    things (id) {
        id -> Integer,
        thing_key -> Text,
    }
}

joinable!(channels -> things (thing_id));
joinable!(config -> things (thing_id));
joinable!(identification -> things (thing_id));
joinable!(sensor_data -> sensors (sensor_id));
joinable!(sensors -> channels (channel_id));
joinable!(sensors -> sensor_types (sensor_types_id));
joinable!(streams -> channels (channel_id));

allow_tables_to_appear_in_same_query!(
    channels,
    config,
    identification,
    identities,
    sensor_data,
    sensor_types,
    sensors,
    streams,
    things,
);
