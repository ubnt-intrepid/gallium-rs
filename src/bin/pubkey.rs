extern crate gallium;
extern crate dotenv;
extern crate diesel;
extern crate clap;
extern crate shlex;

use std::env;
use std::process::{Command, Stdio};
use std::os::unix::process::CommandExt;
use diesel::prelude::*;
use diesel::pg::PgConnection;
use dotenv::dotenv;
use gallium::models::PublicKey;
use gallium::schema::public_keys;

fn build_cli<'a, 'b: 'a>() -> clap::App<'a, 'b> {
    clap::App::new("pubkey")
        .about("manages public keys")
        .setting(clap::AppSettings::SubcommandRequiredElseHelp)
        .setting(clap::AppSettings::VersionlessSubcommands)
        .subcommand(
            clap::SubCommand::with_name("access")
                .about("Authenticated access for SSH command execution")
                .arg_from_usage("--user-id=[user-id]  'User ID'"),
        )
        .subcommand(
            clap::SubCommand::with_name("show")
                .about("Show the list of public keys")
                .arg_from_usage(
                    "--database-url=[url]  'Database URL which contains public keys'",
                ),
        )
}

fn main() {
    dotenv().ok();
    let ref matches = build_cli().get_matches();
    match matches.subcommand() {
        ("access", Some(m)) => access(m),
        ("show", Some(m)) => show(m),
        _ => unreachable!(),
    }
}

fn access(_m: &clap::ArgMatches) {
    // let _user_id: i32 = m.value_of("user-id").and_then(|s| s.parse().ok()).unwrap();
    let command = env::var("SSH_ORIGINAL_COMMAND")
        .ok()
        .and_then(|s| shlex::split(&s))
        .unwrap_or_default();
    if command.len() == 0 {
        println!("failed to parse SSH_ORIGINAL_COMMAND");
        std::process::exit(1);
    }
    let err = Command::new(&command[0])
        .args(&command[1..])
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .exec();
    panic!("{:?}", err)
}

fn show(m: &clap::ArgMatches) {
    let database_url = m.value_of("database-url")
        .map(ToOwned::to_owned)
        .unwrap_or_else(|| env::var("DATABASE_URL").unwrap());

    let conn = PgConnection::establish(&database_url).unwrap();
    let keys: Vec<PublicKey> = public_keys::table.load(&conn).unwrap();
    for key in keys {
        println!("command=\"/opt/gallium/bin/pubkey access\" {}", key.key);
    }
}
