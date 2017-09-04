extern crate rand;

use std::process::{Command, Output};
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
        let status = Command::new("rsync")
            .args(&[
                "--no-motd",
                "--recursive",
                "--times",
                "--keep-dirlinks",
                "--safe-links",
                "--copy-links",
                "--hard-links",
                "--delete-after",
                "--delay-updates",
                "--delete-excluded",
                "--exclude=**spyder3-3.1.4*",
                mirror,
                "/data/arch",
            ])
            .status()
            .expect("failed to execute rsync");
        if status.success() {
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
            .expect("failed to execute ipfs add");
        assert_cmd_output("ipfs add", &ipfs_out);
        let ipfs_stdout = String::from_utf8_lossy(&ipfs_out.stdout);
        ipfs_stdout
            .lines()
            .last()
            .expect("No stdout from ipfs add")
            .to_string()
    };
    println!(">>> ipfs hash: `{}`", ipfs_hash);

    let mut unpin_failed_hashes = Vec::new();
    {
        let hashes_file = OpenOptions::new()
            .read(true)
            .open("/data/pacman-ipfs-adder-hashes");
        if let Ok(mut hashes_file) = hashes_file {
            let mut content = String::new();
            hashes_file
                .read_to_string(&mut content)
                .expect("failed reading hash file");
            for h in content.lines().filter(|h| !h.is_empty() && h != &ipfs_hash) {
                println!(">>> unpinning: `{}`", h);
                let status = Command::new("ipfs")
                    .args(&["pin", "rm", h])
                    .status()
                    .expect("failed to execute ipfs pin rm");
                if !status.success() {
                    unpin_failed_hashes.push(h.to_string());
                    println!(">>> unpin failed on `{}`", h);
                }
            }
        }
    }

    {
        let mut hashes_file = OpenOptions::new()
            .write(true)
            .truncate(true)
            .create(true)
            .open("/data/pacman-ipfs-adder-hashes")
            .expect("failed opening hash file");
        writeln!(
            hashes_file,
            "{}\n{}",
            ipfs_hash,
            unpin_failed_hashes.join("\n")
        ).expect("failed writing hash file");
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
            .output()
            .expect("failed to execute ipfs publish");
        assert_cmd_output("ipfs publish", &ipfs_out);
    }
    println!(">>> done");
}
