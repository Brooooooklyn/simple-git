extern crate napi_build;

fn main() {
  napi_build::setup();
  let compile_target = std::env::var("TARGET").unwrap();
  match compile_target.as_str() {
    "x86_64-unknown-linux-gnu" => {
      println!("cargo:rustc-link-search=/usr/lib");
    }
    "armv7-unknown-linux-gnueabihf" => {
      const CROSS_LIB_PATH: &str = "/usr/lib/gcc-cross/arm-linux-gnueabihf";
      if let Ok(version) = std::process::Command::new("ls")
        .arg(CROSS_LIB_PATH)
        .output()
        .map(|o| String::from_utf8(o.stdout).unwrap().trim().to_string())
      {
        println!("cargo:rustc-link-search={CROSS_LIB_PATH}/{version}");
      };
    }
    _ => {}
  }
}
