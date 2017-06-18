extern crate gallium;
extern crate dotenv;
extern crate diesel;

use std::env;
use std::io;
use diesel::prelude::*;
use diesel::pg::PgConnection;
use dotenv::dotenv;
use gallium::models::{PublicKey, NewPublicKey};
use gallium::schema::public_keys;

fn main() {
    dotenv().ok();
    let conn = PgConnection::establish(&env::var("DATABASE_URL").unwrap()).unwrap();

    println!("[reading public key from stdin...]");
    let mut pubkey = Vec::new();
    io::copy(&mut io::stdin(), &mut pubkey).unwrap();
    let pubkey = String::from_utf8_lossy(&pubkey).into_owned();

    let new_pubkey = NewPublicKey { key: &pubkey };
    let pub_key: PublicKey = diesel::insert(&new_pubkey)
        .into(public_keys::table)
        .get_result(&conn)
        .unwrap();
    println!("[inserted] {:?}", pub_key);
}
