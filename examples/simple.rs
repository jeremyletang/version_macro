#![feature(plugin)]
#![plugin(version_macro)]

mod version {
    infer_version!();
}

fn main() {
    println!("{}", version::format());
}
