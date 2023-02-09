extern crate napi_build;

fn main() {
  napi_build::setup();
  let compile_target = std::env::var("TARGET").unwrap();
  if compile_target == "x86_64-unknown-linux-gnu" {
    println!("cargo:rustc-link-search=/usr/x86_64-unknown-linux-gnu/lib");
  }
}
