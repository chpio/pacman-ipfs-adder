extern crate rand;

use std::process::{Command, Output, Stdio};
use std::fs::OpenOptions;
use std::io::prelude::*;
use rand::Rng;

const MIRRORS: &[&str] = &[
    "rsync://pkg.adfinis-sygroup.ch/archlinux/",
    "rsync://mirror.23media.de/archlinux/",
    "rsync://mirror.f4st.host/archlinux/",
];

fn assert_cmd_output(name: &str, o: &Output) {
    if !o.status.success() {
        panic!(
            "`{}` exited with status: {}\n{}",
            name,
            o.status,
            String::from_utf8_lossy(&o.stderr)
        );
    }
}

fn main() {
    let mut mirrors = MIRRORS.to_vec();
    rand::thread_rng().shuffle(&mut mirrors);
    for mirror in mirrors.iter() {
        println!(">>> rsyncing from `{}`...", mirror);
        let rsync_out = Command::new("rsync")
            .args(&[
                "--no-motd",
                "--recursive",
                "--times",
                "--safe-links",
                "--copy-links",
                "--hard-links",
                "--delete-after",
                "--delay-updates",
                mirror,
                "/data/arch",
            ])
            .stdout(Stdio::inherit())
            .output()
            .unwrap();
        if rsync_out.status.success() {
            break;
        } else {
            println!(">>> rsync failed");
            if mirror == mirrors.last().unwrap() {
                panic!("all mirrors failed");
            }
        }
    }

    println!(">>> adding to ipfs...");
    let ipfs_hash = {
        let ipfs_out = Command::new("ipfs")
            .args(&["add", "--quiet", "--recursive", "/data/arch"])
            .output()
            .unwrap();
        assert_cmd_output("ipfs add", &ipfs_out);
        let ipfs_stdout = String::from_utf8_lossy(&ipfs_out.stdout);
        ipfs_stdout
            .lines()
            .last()
            .expect("No stdout from ipfs add")
            .to_string()
    };
    println!(">>> ipfs hash: {:?}", ipfs_hash);

    {
        let hashes_file = OpenOptions::new()
            .read(true)
            .open("/data/pacman-ipfs-adder-hashes");
        if let Ok(mut hashes_file) = hashes_file {
            let mut content = String::new();
            hashes_file.read_to_string(&mut content).unwrap();
            for h in content.lines().filter(|h| h != &ipfs_hash) {
                println!(">>> unpinning: {:?}", h);
                Command::new("ipfs")
                    .args(&["pin", "rm", h])
                    .stdout(Stdio::inherit())
                    .stderr(Stdio::inherit())
                    .spawn()
                    .unwrap()
                    .wait()
                    .unwrap();
            }
        }
    }

    {
        let mut hashes_file = OpenOptions::new()
            .write(true)
            .truncate(true)
            .create(true)
            .open("/data/pacman-ipfs-adder-hashes")
            .unwrap();
        writeln!(hashes_file, "{}", ipfs_hash).unwrap();
    }

    println!(">>> publishing to ipns...");
    {
        let ipfs_out = Command::new("ipfs")
            .args(&[
                "name",
                "publish",
                "--lifetime=12h",
                "--ttl=1h",
                &ipfs_hash,
            ])
            .stdout(Stdio::inherit())
            .output()
            .unwrap();
        assert_cmd_output("ipfs publish", &ipfs_out);
    }
    println!(">>> done");
}
