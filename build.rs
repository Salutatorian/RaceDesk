fn main() {
    println!("cargo:rerun-if-changed=assets/racedesk.ico");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        winresource::WindowsResource::new()
            .set_icon("assets/racedesk.ico")
            .set("FileDescription", "RaceDesk iRacing desktop widget")
            .set("ProductName", "RaceDesk")
            .set("OriginalFilename", "RaceDesk.exe")
            .compile()
            .expect("Unable to embed RaceDesk Windows icon");
    }
}
