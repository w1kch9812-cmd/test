load("@crates//:defs.bzl", "aliases", "all_crate_deps")
load("@rules_rust//rust:defs.bzl", "rust_binary", "rust_library", "rust_test")

def gongzzang_rust_library_with_unit_test(
        name,
        srcs,
        crate_name = None,
        compile_data = None,
        internal_deps = None,
        internal_dev_deps = None,
        rustc_env = None):
    """Defines the standard Gongzzang Rust library plus unit-test target."""
    if crate_name == None:
        crate_name = name
    if internal_deps == None:
        internal_deps = []
    if internal_dev_deps == None:
        internal_dev_deps = []
    if compile_data == None:
        compile_data = []
    if rustc_env == None:
        rustc_env = {}

    rust_library(
        name = name,
        aliases = aliases(),
        crate_name = crate_name,
        crate_root = "src/lib.rs",
        compile_data = compile_data,
        edition = "2021",
        proc_macro_deps = all_crate_deps(proc_macro = True),
        rustc_env = rustc_env,
        srcs = srcs,
        deps = internal_deps + all_crate_deps(normal = True),
    )

    rust_test(
        name = name + "_unit_test",
        aliases = aliases(
            normal_dev = True,
            proc_macro_dev = True,
        ),
        crate = ":" + name,
        edition = "2021",
        proc_macro_deps = all_crate_deps(proc_macro_dev = True),
        rustc_env = rustc_env,
        deps = internal_dev_deps + all_crate_deps(normal_dev = True),
    )

def gongzzang_rust_binary_with_unit_test(
        name,
        srcs,
        crate_root,
        crate_name = None,
        compile_data = None,
        internal_deps = None,
        internal_dev_deps = None,
        rustc_env = None):
    """Defines the standard Gongzzang Rust binary plus unit-test target."""
    if crate_name == None:
        crate_name = name
    if internal_deps == None:
        internal_deps = []
    if internal_dev_deps == None:
        internal_dev_deps = []
    if compile_data == None:
        compile_data = []
    if rustc_env == None:
        rustc_env = {}

    rust_binary(
        name = name,
        aliases = aliases(),
        crate_name = crate_name,
        crate_root = crate_root,
        compile_data = compile_data,
        edition = "2021",
        proc_macro_deps = all_crate_deps(proc_macro = True),
        rustc_env = rustc_env,
        srcs = srcs,
        deps = internal_deps + all_crate_deps(normal = True),
    )

    rust_test(
        name = name + "_unit_test",
        aliases = aliases(
            normal = True,
            normal_dev = True,
            proc_macro = True,
            proc_macro_dev = True,
        ),
        crate_root = crate_root,
        edition = "2021",
        proc_macro_deps = all_crate_deps(proc_macro = True) + all_crate_deps(proc_macro_dev = True),
        rustc_env = rustc_env,
        srcs = srcs,
        deps = internal_deps + internal_dev_deps + all_crate_deps(normal = True) + all_crate_deps(normal_dev = True),
    )
