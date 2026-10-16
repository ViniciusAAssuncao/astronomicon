fn main() {
    cxx_build::bridge("src/lib.rs")
        .include("include")
        .file("src/forecast_progress.cpp")
        .std("c++17")
        .compile("rocketcon-cxx");
    println!("cargo:rerun-if-changed=src/lib.rs");
    println!("cargo:rerun-if-changed=src/forecast_progress.cpp");
    println!("cargo:rerun-if-changed=include/rocketcon_forecast_progress.hpp");
}
