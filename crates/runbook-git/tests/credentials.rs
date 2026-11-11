//! A remote that asks for credentials makes Git fail at once instead of waiting for a
//! password nobody can type. The remote is a local HTTP listener that answers every
//! request with `401`, so the test needs no network.

mod common;

use std::io::{BufRead, BufReader, Write};
use std::net::TcpListener;
use std::thread;
use std::time::{Duration, Instant};

use common::Fixture;
use runbook_git::{GitError, Repository};

fn serve_unauthorized() -> String {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind a local port");
    let address = listener.local_addr().expect("local address");
    // Detached: it lives until the test process exits.
    thread::spawn(move || {
        for stream in listener.incoming().flatten() {
            let mut reader = BufReader::new(&stream);
            let mut line = String::new();
            while reader.read_line(&mut line).is_ok_and(|n| n > 0) && line != "\r\n" {
                line.clear();
            }
            let _ = (&stream).write_all(
                b"HTTP/1.1 401 Unauthorized\r\nWWW-Authenticate: Basic realm=\"test\"\r\n\
                  Content-Length: 0\r\nConnection: close\r\n\r\n",
            );
        }
    });
    format!("http://{address}/repo.git")
}

#[test]
fn a_credential_prompt_fails_instead_of_hanging() {
    let f = Fixture::new();
    f.git(
        &f.work,
        &["remote", "set-url", "origin", &serve_unauthorized()],
    );

    let started = Instant::now();
    let err = Repository::new(&f.work, &f.env).fetch().unwrap_err();
    assert!(matches!(err, GitError::Failed(_)), "{err:?}");
    assert!(started.elapsed() < Duration::from_secs(30));
}
