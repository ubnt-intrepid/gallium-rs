extern crate gallium;
extern crate diesel;
extern crate clap;
extern crate shlex;

use std::env;
use std::io::Write;
use std::process::Command;
use std::os::unix::process::CommandExt;
use std::path::Path;
use diesel::prelude::*;
use gallium::models::{Project, PublicKey};
use gallium::schema::public_keys;
use gallium::config::Config;
use gallium::db::DB;
use gallium::models::repository::open_repository;


fn build_cli<'a, 'b: 'a>() -> clap::App<'a, 'b> {
    clap::App::new("pubkey")
        .about("manages public keys")
        .setting(clap::AppSettings::SubcommandRequiredElseHelp)
        .setting(clap::AppSettings::VersionlessSubcommands)
        .subcommand(
            clap::SubCommand::with_name("access")
                .about("Authenticated access for SSH command execution")
                .arg_from_usage("--user-id=<user-id>  'User ID'"),
        )
        .subcommand(clap::SubCommand::with_name("show").about(
            "Show the list of public keys",
        ))
}

fn main() {
    let ref matches = build_cli().get_matches();
    let err = match matches.subcommand() {
        ("access", Some(m)) => access(m),
        ("show", Some(m)) => show(m),
        _ => unreachable!(),
    };
    if let Err(err) = err {
        println!("Failed with: {}", err);
        std::process::exit(1);
    }
}

fn access(m: &clap::ArgMatches) -> Result<(), String> {
    let config = Config::load().unwrap();
    let db = DB::new(&config.database_url).unwrap();

    let s = env::var("SSH_ORIGINAL_COMMAND").map_err(
        |err| err.to_string(),
    )?;
    let (action, user, project) = parse_ssh_command(&s)?;

    let (_user, project, repo) = open_repository(&db, &config, &user, &project)
        .map_err(|err| err.to_string())?
        .ok_or_else(|| "Failed to open repository".to_owned())?;

    let user_id = m.value_of("user-id").and_then(|s| s.parse().ok()).unwrap();
    check_scope(&action, user_id, &project)?;

    let err = Command::new(action)
        .arg(repo.path().to_str().unwrap())
        .exec();
    let _ = writeln!(&mut std::io::stderr(), "failed to exec: {:?}", err);
    std::process::exit(1);
}

fn show(_m: &clap::ArgMatches) -> Result<(), String> {
    let config = Config::load().unwrap();
    let db = DB::new(&config.database_url).unwrap();
    let conn = db.get_db_conn().unwrap();

    let keys: Vec<PublicKey> = public_keys::table.load(&*conn).unwrap();
    for key in keys {
        println!(
            "command=\"/opt/gallium/bin/pubkey access --user-id={}\" {}",
            key.user_id,
            key.key
        );
    }

    Ok(())
}

fn parse_ssh_command(s: &str) -> Result<(String, String, String), String> {
    let command = shlex::split(s).ok_or_else(|| {
        "failed to parse SSH_ORIGINAL_COMMAND".to_string()
    })?;

    if command.len() < 1 {
        return Err("command is not given".to_string());
    } else if command.len() < 2 {
        return Err("Missing repository".to_string());
    }

    // validate action
    let action = &command[0];
    if action != "git-receive-pack" && action != "git-upload-pack" && action != "git-upload-archive" {
        return Err("Permission denied".to_string());
    }

    // validate repository
    let repository = &command[1];
    if Path::new(repository).is_absolute() || repository.starts_with("./") || repository.starts_with("../") {
        return Err("incorrect repository path".to_string());
    }

    let elems: Vec<_> = repository.split("/").collect();
    if elems.len() != 2 {
        return Err("Incorrect repository path".to_owned());
    }

    let user = &elems[0];
    let project = &elems[1];

    if !project.ends_with(".git") {
        return Err("The repository URL should be end with '.git'".to_owned());
    }
    let project = project.trim_right_matches(".git");

    Ok((action.to_owned(), (*user).to_owned(), project.to_owned()))
}

fn check_scope(action: &str, user_id: i32, project: &Project) -> Result<(), String> {
    match action {
        "git-receive-pack" => {
            if project.user_id != user_id {
                return Err("Permission denied".to_string());
            }
        }
        _ => (),
    }
    Ok(())
}
