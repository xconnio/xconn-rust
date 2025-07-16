fn main() {
    // when developing xconn itself, allow to install both variants
    // of the library.
    let crate_name = std::env::var("CARGO_PKG_NAME").unwrap();
    if crate_name == "xconn" {
        return;
    }

    let sync = std::env::var("CARGO_FEATURE_SYNC").is_ok();
    let async_ = std::env::var("CARGO_FEATURE_ASYNC").is_ok();

    if sync && async_ {
        panic!(
            "\n\
==============================================================\n\
ERROR: Features 'sync' and 'async' cannot be enabled together.\n\
\n\
To use the sync variant of xconn, add this to your Cargo.toml:\n\
    xconn = {{ version = \"...\", features = [\"sync\"], default-features = false }}\n\
\n\
To use the async variant (default), use:\n\
    xconn = {{ version = \"...\", features = [\"async\"] }}\n\
\n\
=============================================================="
        );
    }
}
