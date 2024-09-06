pub struct Cfg;

impl Cfg {
    pub fn link() {
        println!("cargo::rustc-check-cfg=cfg(cross_platform, values(any()))");
    }
}
