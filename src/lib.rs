// Copyright 2016 Jeremy Letang.
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

#![feature(plugin_registrar, rustc_private, plugin)]
#![plugin(quasi_macros)]

extern crate aster;
extern crate git2;
extern crate quasi;
extern crate rustc_plugin;
extern crate time;
extern crate toml;
extern crate syntax;

use std::fs::File;
use std::io::Read;

use aster::lit::LitBuilder;
use git2::Repository;
use toml::Value;
use syntax::ast::*;
use syntax::codemap::Span;
use syntax::ext::base::*;
use syntax::ptr::P;
use syntax::util::small_vector::SmallVector;


pub struct Version {
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
}

impl Version {
    pub fn new(s: &str) -> Version {
        let split_v: Vec<&str> = s.split(".").collect();
        Version {
            major: u32::from_str_radix(split_v[0], 10).unwrap(),
            minor: u32::from_str_radix(split_v[1], 10).unwrap(),
            patch: u32::from_str_radix(split_v[2], 10).unwrap(),
        }
    }

    pub fn as_string(&self) -> String {
        format!("{}.{}.{}", self.major, self.minor, self.patch)
    }
}

fn read_infos_from_toml() -> (String, String) {
    let mut input = String::new();
    File::open("Cargo.toml").and_then(|mut f| {
        f.read_to_string(&mut input)
    }).unwrap();
    let mut parser = toml::Parser::new(&input);
    // lets assume cargo can validate himself that Cargo.toml is valid
    let cargo_toml = parser.parse().unwrap();
    let package = match cargo_toml.get(&"package".to_string()).unwrap() {
        &Value::Table(ref t) => t.clone(),
        _ => unreachable!()
    };
    let version = match package.get(&"version".to_string()).unwrap() {
        &Value::String(ref v) => v.clone(),
        _ => unreachable!()
    };
    let bin_name = match package.get(&"name".to_string()).unwrap() {
        &Value::String(ref v) => v.clone(),
        _ => unreachable!()
    };
    return (version, bin_name);
}

fn make_build_number() -> String {
    fn add_zero(n: i32) -> String {
        if n < 10 {
            format!("0{}", n)
        } else {
            format!("{}", n)
        }
    }

    let t = time::now();
    format!(
        "{}{}{}{}{}{}",
        // year since 1900
        t.tm_year + 1900,
        // begin at 0
        add_zero(t.tm_mon + 1),
        add_zero(t.tm_mday),
        add_zero(t.tm_hour),
        add_zero(t.tm_min),
        add_zero(t.tm_sec)
    )
}

fn make_git_sha1() -> String {
    // we assume we are in a git repository
    let repo = Repository::open(".").ok().unwrap();
    let revspec = repo.revparse("HEAD").unwrap();
    format!("{}", revspec.from().unwrap().id())
}

fn make_func_items(cx: &mut ExtCtxt) -> SmallVector<P<Item>> {
    let fn_format = quote_item!(cx,
        pub fn format() -> String {
            format!("{} version {} (git rev {}; build {})",
                BIN_NAME, VERSION, &GIT_SHA1[..4], BUILD_NUMBER
            )
        }
    ).unwrap();

    let fn_format_full = quote_item!(cx,
        pub fn format_full() -> String {
            format!("{} version {}\ngit revision {}\nbuild {})",
                BIN_NAME, VERSION, GIT_SHA1, BUILD_NUMBER)
        }
    ).unwrap();

    SmallVector::many(vec![fn_format, fn_format_full])
}

fn make_const_items(cx: &mut ExtCtxt,
                    v: Version,
                    build_nb: &str,
                    sha1: &str,
                    bin_name: &str) -> SmallVector<P<Item>> {
    let lit_version = (*LitBuilder::new().str(&*v.as_string())).clone();
    let version = quote_item!(cx,
        #[allow(dead_code)]
        pub const VERSION: &'static str = $lit_version;).unwrap();
    let lit_major = (*LitBuilder::new().u32(v.major)).clone();
    let major = quote_item!(cx,
        #[allow(dead_code)]
        pub const VERSION_MAJOR: u32 = $lit_major;).unwrap();
    let lit_minor = (*LitBuilder::new().u32(v.minor)).clone();
    let minor = quote_item!(cx,
        #[allow(dead_code)]
        pub const VERSION_MINOR: u32 = $lit_minor;).unwrap();
    let lit_patch = (*LitBuilder::new().u32(v.patch)).clone();
    let patch = quote_item!(cx,
        #[allow(dead_code)]
        pub const VERSION_PATCH: u32 = $lit_patch;).unwrap();
    let lit_sha1 = (*LitBuilder::new().str(sha1)).clone();
    let sha1 = quote_item!(cx,
        #[allow(dead_code)]
        pub const GIT_SHA1: &'static str = $lit_sha1;).unwrap();
    let lit_build_nb = (*LitBuilder::new().str(build_nb)).clone();
    let build_nb = quote_item!(cx,
        #[allow(dead_code)]
        pub const BUILD_NUMBER: &'static str = $lit_build_nb;).unwrap();
    let lit_bin_name = (*LitBuilder::new().str(bin_name)).clone();
    let bin_name = quote_item!(cx,
        #[allow(dead_code)]
        pub const BIN_NAME: &'static str = $lit_bin_name;).unwrap();

    SmallVector::many(vec![
        version, major, minor, patch, sha1, build_nb, bin_name
    ])
}

pub fn expand_version<'cx>(
    cx: &'cx mut ExtCtxt,
    _: Span,
    _: &[TokenTree]
) -> Box<MacResult+'cx> {
    // get information
    let (str_version, bin_name) = read_infos_from_toml();
    let build_nb = make_build_number();
    let version = Version::new(&str_version);
    let sha1 = make_git_sha1();
    // create items
    let mut items = SmallVector::zero();
    items.push_all(make_const_items(cx, version, &build_nb, &sha1, &bin_name));
    items.push_all(make_func_items(cx));

    return MacEager::items(items);
}


#[plugin_registrar]
pub fn register(reg: &mut rustc_plugin::Registry) {
    reg.register_macro("infer_version", expand_version);
}
