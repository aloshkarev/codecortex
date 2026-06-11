//! Prost types generated from `docs/a2a.proto` (normative wire schema).

pub mod wkt;

pub mod google;

pub mod lf {
    pub mod a2a {
        pub mod v1 {
            include!(concat!(env!("OUT_DIR"), "/lf.a2a.v1.rs"));
        }
    }
}
