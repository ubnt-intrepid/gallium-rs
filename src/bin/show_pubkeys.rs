extern crate gallium;
extern crate dotenv;
extern crate diesel;
extern crate clap;

use std::env;
use diesel::prelude::*;
use diesel::pg::PgConnection;
use dotenv::dotenv;
use gallium::models::PublicKey;
use gallium::schema::public_keys;

fn build_cli<'a, 'b: 'a>() -> clap::App<'a, 'b> {
    clap::App::new("show_pubkeys")
        .about("Show the list of public keys")
        .arg_from_usage("--database-url=[url]  ''")
}

fn main() {
    dotenv().ok();
    let ref matches = build_cli().get_matches();

    let database_url = matches
        .value_of("database-url")
        .map(ToOwned::to_owned)
        .unwrap_or_else(|| env::var("DATABASE_URL").unwrap());

    let conn = PgConnection::establish(&database_url).unwrap();
    let keys: Vec<PublicKey> = public_keys::table.load(&conn).unwrap();
    for key in keys {
        print!("{}", key.key);
    }
}
