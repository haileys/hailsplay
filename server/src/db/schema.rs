// @generated automatically by Diesel CLI.

diesel::table! {
    archived_media (id) {
        id -> Integer,
        path -> Text,
        canonical_url -> Text,
        archived_at -> Text,
        stream_uuid -> Text,
        thumbnail_id -> Nullable<Integer>,
        metadata -> Text,
    }
}

diesel::table! {
    asset_blobs (digest_sha256) {
        digest_sha256 -> Text,
        blob -> Binary,
    }
}

diesel::table! {
    assets (id) {
        id -> Integer,
        filename -> Text,
        content_type -> Text,
        digest_sha256 -> Text,
    }
}

diesel::table! {
    radio_stations (id) {
        id -> Integer,
        name -> Text,
        icon_id -> Integer,
        stream_url -> Text,
    }
}

diesel::table! {
    schema_migrations (version) {
        version -> Nullable<Text>,
    }
}

diesel::joinable!(archived_media -> assets (thumbnail_id));
diesel::joinable!(assets -> asset_blobs (digest_sha256));
diesel::joinable!(radio_stations -> assets (icon_id));

diesel::allow_tables_to_appear_in_same_query!(
    archived_media,
    asset_blobs,
    assets,
    radio_stations,
    schema_migrations,
);
