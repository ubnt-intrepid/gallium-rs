extern crate gallium;
extern crate dotenv;
extern crate diesel;
extern crate clap;

use std::env;
use std::fs::OpenOptions;
use std::io::{self, Read};
use diesel::prelude::*;
use diesel::pg::PgConnection;
use dotenv::dotenv;
use gallium::models::{PublicKey, NewPublicKey};
use gallium::schema::public_keys;

fn build_cli<'a, 'b: 'a>() -> clap::App<'a, 'b> {
    clap::App::new("pubkey")
        .about("manages public keys")
        .setting(clap::AppSettings::SubcommandRequiredElseHelp)
        .setting(clap::AppSettings::VersionlessSubcommands)
        .subcommand(
            clap::SubCommand::with_name("register")
                .about("Register new public key to database")
                .arg_from_usage("--user-id=<user-id>    'User ID'")
                .arg_from_usage("[filename]             'Path of target pubkey'"),
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
        ("register", Some(m)) => register(m),
        ("show", Some(m)) => show(m),
        _ => unreachable!(),
    }
}


fn register(m: &clap::ArgMatches) {
    dotenv().ok();
    let conn = PgConnection::establish(&env::var("DATABASE_URL").unwrap()).unwrap();

    let user_id: i32 = m.value_of("user-id").and_then(|s| s.parse().ok()).unwrap();

    let filename = m.value_of("filename").unwrap_or("-");
    let mut f: Box<Read> = if filename == "-" {
        Box::new(io::stdin())
    } else {
        Box::new(OpenOptions::new().read(true).open(filename).unwrap())
    };
    let mut pubkey = Vec::new();
    io::copy(&mut f, &mut pubkey).unwrap();
    let key = &String::from_utf8_lossy(&pubkey).trim().to_owned();

    let new_pubkey = NewPublicKey { user_id, key };
    let pub_key: PublicKey = diesel::insert(&new_pubkey)
        .into(public_keys::table)
        .get_result(&conn)
        .unwrap();
    println!("[inserted] {:?}", pub_key);
}


fn show(m: &clap::ArgMatches) {
    let database_url = m.value_of("database-url")
        .map(ToOwned::to_owned)
        .unwrap_or_else(|| env::var("DATABASE_URL").unwrap());

    let conn = PgConnection::establish(&database_url).unwrap();
    let keys: Vec<PublicKey> = public_keys::table.load(&conn).unwrap();
    for key in keys {
        println!("{}", key.key);
    }
}
