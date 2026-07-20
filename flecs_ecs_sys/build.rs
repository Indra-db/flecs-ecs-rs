#![allow(clippy::all)]
#![allow(warnings)]

#[cfg(feature = "regenerate_binding")]
fn generate_bindings(c_debug: bool, out_file: &str) {
    use std::{env, path::PathBuf};

    #[derive(Debug)]
    struct CommentsCallbacks;

    impl bindgen::callbacks::ParseCallbacks for CommentsCallbacks {
        fn process_comment(&self, comment: &str) -> Option<String> {
            // 1: trimming the comments
            let comment = comment.trim();
            // 2: brackets do not entail intra-links
            let comment = comment.replace("[", "\\[");
            let comment = comment.replace("]", "\\]");

            // ensure all links are padded with < and >
            let url_re = regex::Regex::new(r"(?P<url>https?://[^\s]+)").unwrap();
            let comment = url_re
                .replace_all(comment.as_str(), |caps: &regex::Captures| {
                    format!("<{}>", &caps["url"])
                })
                .into_owned();

            Some(comment)
        }
    }

    let mut bindings = bindgen::Builder::default()
        .header("src/flecs_rust.h")
        // Only keep things that we've allowlisted rather than
        // recursively keeping nested uses around.
        .allowlist_file(".*src[/\\\\]flecs\\.h")
        .allowlist_file(".*src[/\\\\]flecs_rust\\.h")
        .allowlist_recursively(false);

    // Use appropriate ABI based on target platform
    // WASM doesn't support unwinding, so use "C" ABI
    // Other platforms can use "C-unwind" ABI
    let target = env::var("TARGET").unwrap();
    if target.contains("wasm") {
        bindings = bindings.override_abi(bindgen::Abi::C, ".*");
    } else {
        bindings = bindings.override_abi(bindgen::Abi::CUnwind, ".*");
    }

    let mut bindings = bindings
        // Keep comments and keep all of them, not just doc comments.
        .generate_comments(true)
        // Prefer core::* over std::*
        .use_core()
        // Don't generate a bunch of `link_name` attributes
        .trust_clang_mangling(false)
        .clang_arg("-fparse-all-comments")
        .parse_callbacks(Box::new(CommentsCallbacks))
        .blocklist_item("FLECS_HI_COMPONENT_ID")
        .blocklist_item("FLECS_TERM_COUNT_MAX")
        .blocklist_item("ECS_PAIR")
        .blocklist_item("ECS_OVERRIDE")
        .blocklist_item("ECS_TOGGLE")
        .blocklist_item("ECS_AND")
        // We'll use `libc::FILE` instead.
        .blocklist_type("FILE")
        // These have doc comments that trigger doc tests.
        .blocklist_type("ecs_alert_desc_t")
        .blocklist_function("ecs_http_server_http_request")
        .blocklist_function("ecs_log_enable_timedelta")
        .blocklist_function("ecs_get_world_info")
        // These use va_list
        .blocklist_function("ecs_logv_")
        .blocklist_function("ecs_printv_")
        .blocklist_function("ecs_parser_errorv_")
        .blocklist_function("ecs_strbuf_vappend")
        .blocklist_function("flecs_vasprintf")
        .blocklist_function("ecs_parser_warningv_")
        .layout_tests(false)
        .raw_line("#![allow(clippy::all)]")
        .raw_line("#![allow(warnings)]")
        .raw_line("use super::*;")
        .raw_line("use libc::FILE;")
        .clang_arg("-DFLECS_CUSTOM_BUILD")
        .clang_arg("-DFLECS_CPP");

    #[cfg(feature = "flecs_default_to_uncached_queries")]
    {
        bindings = bindings.clang_arg("-DFLECS_DEFAULT_TO_UNCACHED_QUERIES");
    }

    #[cfg(feature = "flecs_script_math")]
    {
        bindings = bindings.clang_arg("-DFLECS_SCRIPT_MATH");
    }

    #[cfg(feature = "flecs_perf_trace")]
    {
        bindings = bindings.clang_arg("-DFLECS_PERF_TRACE");
    }

    #[cfg(feature = "flecs_module")]
    {
        bindings = bindings.clang_arg("-DFLECS_MODULE");
    }

    #[cfg(feature = "flecs_script")]
    {
        bindings = bindings.clang_arg("-DFLECS_SCRIPT");
    }

    #[cfg(feature = "flecs_stats")]
    {
        bindings = bindings.clang_arg("-DFLECS_STATS");
    }

    #[cfg(feature = "flecs_metrics")]
    {
        bindings = bindings.clang_arg("-DFLECS_METRICS");
    }

    #[cfg(feature = "flecs_alerts")]
    {
        bindings = bindings.clang_arg("-DFLECS_ALERTS");
    }

    #[cfg(feature = "flecs_system")]
    {
        bindings = bindings.clang_arg("-DFLECS_SYSTEM");
    }

    #[cfg(feature = "flecs_pipeline")]
    {
        bindings = bindings.clang_arg("-DFLECS_PIPELINE");
    }

    #[cfg(feature = "flecs_timer")]
    {
        bindings = bindings.clang_arg("-DFLECS_TIMER");
    }

    #[cfg(feature = "flecs_meta")]
    {
        bindings = bindings.clang_arg("-DFLECS_META");
    }

    #[cfg(feature = "flecs_meta_c")]
    {
        bindings = bindings.clang_arg("-DFLECS_META_C");
    }

    #[cfg(feature = "flecs_units")]
    {
        bindings = bindings.clang_arg("-DFLECS_UNITS");
    }

    #[cfg(feature = "flecs_json")]
    {
        bindings = bindings.clang_arg("-DFLECS_JSON");
    }

    #[cfg(feature = "flecs_doc")]
    {
        bindings = bindings.clang_arg("-DFLECS_DOC");
    }

    #[cfg(feature = "flecs_log")]
    {
        bindings = bindings.clang_arg("-DFLECS_LOG");
    }

    #[cfg(feature = "flecs_app")]
    {
        bindings = bindings.clang_arg("-DFLECS_APP");
    }

    #[cfg(feature = "flecs_os_api_impl")]
    {
        bindings = bindings.clang_arg("-DFLECS_OS_API_IMPL");
    }

    #[cfg(not(feature = "flecs_os_api_impl"))]
    {
        bindings = bindings.clang_arg("-DFLECS_NO_OS_API_IMPL");
    }

    #[cfg(feature = "flecs_http")]
    {
        bindings = bindings.clang_arg("-DFLECS_HTTP");
    }

    #[cfg(feature = "flecs_rest")]
    {
        bindings = bindings.clang_arg("-DFLECS_REST");
    }

    #[cfg(feature = "flecs_journal")]
    {
        bindings = bindings.clang_arg("-DFLECS_JOURNAL");
    }

    let term_count_max = if cfg!(feature = "flecs_term_count_64") {
        64
    } else {
        32 // default value
    };

    let term_arg = format!("-DFLECS_TERM_COUNT_MAX={}", term_count_max);

    bindings = bindings
        .clang_arg(term_arg)
        .raw_line(format!("pub const FLECS_TERM_COUNT_MAX: u32 = {};", term_count_max).as_str());

    // The debug/release variants must only differ in FLECS_DEBUG vs
    // FLECS_NDEBUG: several structs (ecs_ref_t, ecs_map_t, ...) carry
    // trailing debug-only fields, so each compiled C profile needs a
    // layout-matched bindings file (selected in lib.rs via `flecs_c_release`).
    bindings = if c_debug {
        bindings.clang_arg("-DFLECS_DEBUG")
    } else {
        bindings.clang_arg("-DFLECS_NDEBUG").clang_arg("-DNDEBUG")
    };

    let bindings = bindings.generate().expect("Unable to generate bindings");

    let crate_root: PathBuf = env::var("CARGO_MANIFEST_DIR").unwrap().into();
    bindings.write_to_file(crate_root.join(out_file)).unwrap();
}

fn main() {
    // Tell cargo to invalidate the built crate whenever the sources change
    println!("cargo:rerun-if-changed=src/flecs.h");
    println!("cargo:rerun-if-changed=src/flecs.c");
    println!("cargo:rerun-if-changed=src/flecs_rust.h");
    println!("cargo:rerun-if-changed=src/flecs_rust.c");
    println!("cargo:rerun-if-changed=build.rs");

    // Decide the C build profile from the target profile, not from this build
    // script's own `debug_assertions` (build scripts can be compiled with a
    // different profile than the target). lib.rs selects the layout-matched
    // bindings module via the `flecs_c_release` cfg.
    let profile = std::env::var("PROFILE").unwrap_or_default();
    let force_release = std::env::var("CARGO_FEATURE_FORCE_BUILD_RELEASE").is_ok();
    let force_debug = std::env::var("CARGO_FEATURE_FORCE_BUILD_DEBUG").is_ok();
    let release_c = force_release || (profile == "release" && !force_debug);

    println!("cargo::rustc-check-cfg=cfg(flecs_c_release)");
    if release_c {
        println!("cargo:rustc-cfg=flecs_c_release");
    }

    #[cfg(not(feature = "disable_build_c"))]
    {
        let mut build = cc::Build::new();

        build
            .file("src/flecs_rust.c") // This includes flecs.c
            .warnings(true)
            .extra_warnings(true)
            .define("FLECS_CUSTOM_BUILD", None)
            .define("FLECS_CPP", None);

        #[cfg(feature = "flecs_perf_trace")]
        build.define("FLECS_PERF_TRACE", None);

        #[cfg(feature = "flecs_module")]
        build.define("FLECS_MODULE", None);

        #[cfg(feature = "flecs_script")]
        build.define("FLECS_SCRIPT", None);

        #[cfg(feature = "flecs_stats")]
        build.define("FLECS_STATS", None);

        #[cfg(feature = "flecs_metrics")]
        build.define("FLECS_METRICS", None);

        #[cfg(feature = "flecs_alerts")]
        build.define("FLECS_ALERTS", None);

        #[cfg(feature = "flecs_system")]
        build.define("FLECS_SYSTEM", None);

        #[cfg(feature = "flecs_pipeline")]
        build.define("FLECS_PIPELINE", None);

        #[cfg(feature = "flecs_timer")]
        build.define("FLECS_TIMER", None);

        #[cfg(feature = "flecs_meta")]
        build.define("FLECS_META", None);

        #[cfg(feature = "flecs_meta_c")]
        build.define("FLECS_META_C", None);

        #[cfg(feature = "flecs_units")]
        build.define("FLECS_UNITS", None);

        #[cfg(feature = "flecs_json")]
        build.define("FLECS_JSON", None);

        #[cfg(feature = "flecs_doc")]
        build.define("FLECS_DOC", None);

        #[cfg(feature = "flecs_log")]
        build.define("FLECS_LOG", None);

        #[cfg(feature = "flecs_app")]
        build.define("FLECS_APP", None);

        #[cfg(feature = "flecs_os_api_impl")]
        build.define("FLECS_OS_API_IMPL", None);

        #[cfg(feature = "flecs_http")]
        build.define("FLECS_HTTP", None);

        #[cfg(feature = "flecs_rest")]
        build.define("FLECS_REST", None);

        #[cfg(feature = "flecs_journal")]
        build.define("FLECS_JOURNAL", None);

        if release_c {
            build
                .opt_level(3)
                .define("NDEBUG", None)
                .define("FLECS_NDEBUG", None)
                .define("flto", None);
        } else {
            build.define("FLECS_DEBUG", None);
        }

        #[cfg(feature = "use_os_alloc")]
        {
            build.define("FLECS_USE_OS_ALLOC", None);
        }

        #[cfg(feature = "flecs_force_enable_ecs_asserts")]
        {
            build.define("FLECS_KEEP_ASSERT", None);
        }

        let term_count_max = if cfg!(feature = "flecs_term_count_64") {
            64
        } else {
            32 // default value
        };

        build.define(
            "FLECS_TERM_COUNT_MAX",
            Some(term_count_max.to_string().as_str()),
        );

        // The Rust callback trampolines use the `extern "C-unwind"` ABI so that a
        // panic in a user system/observer/hook can unwind back to the Rust caller
        // (e.g. the `catch_unwind` in the app run loop). Unwinding *through* these
        // C frames is only defined if the C code carries unwind tables, so emit
        // them explicitly. On wasm unwinding is unsupported (the trampolines use
        // plain `extern "C"`), and MSVC/other compilers that don't accept these
        // flags simply ignore them via `flag_if_supported`.
        #[cfg(not(target_family = "wasm"))]
        {
            build.flag_if_supported("-fexceptions");
            build.flag_if_supported("-fasynchronous-unwind-tables");
        }

        build.compile("flecs");

        //TODO C might complain about unused functions when disabling certain features, turn the warning off?

        #[cfg(feature = "regenerate_binding")]
        {
            generate_bindings(true, "src/bindings.rs");
            generate_bindings(false, "src/bindings_release.rs");
        }
    }
}
