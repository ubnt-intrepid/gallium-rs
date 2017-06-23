extern crate gallium;
extern crate diesel;
extern crate clap;
extern crate shlex;

use std::env;
use std::process::Command;
use std::os::unix::process::CommandExt;
use std::path::Path;
use diesel::prelude::*;
use gallium::models::PublicKey;
use gallium::schema::public_keys;
use gallium::app::App;
use gallium::config::Config;

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
        .subcommand(clap::SubCommand::with_name("show").about(
            "Show the list of public keys",
        ))
}

fn main() {
    let ref matches = build_cli().get_matches();
    match matches.subcommand() {
        ("access", Some(m)) => access(m),
        ("show", Some(m)) => show(m),
        _ => unreachable!(),
    }
}

fn access(_m: &clap::ArgMatches) {
    // let _user_id: i32 = m.value_of("user-id").and_then(|s| s.parse().ok()).unwrap();
    let ssh_original_command =
        env::var("SSH_ORIGINAL_COMMAND").expect("Not found: 'SSH_ORIGINAL_COMMAND'");
    let command =
        shlex::split(&ssh_original_command).expect("failed to parse SSH_ORIGINAL_COMMAND");
    if command.len() < 1 {
        panic!("command is not given");
    } else if command.len() < 2 {
        panic!("Missing repository");
    }

    // validate action
    let action = &command[0];
    if action != "git-receive-pack" && action != "git-upload-pack" &&
        action != "git-upload-archive"
    {
        panic!("Permission denied.");
    }

    // validate repository
    let repository = &command[1];
    if Path::new(repository).is_absolute() || repository.starts_with("./") ||
        repository.starts_with("../")
    {
        panic!("incorrect repository path");
    }
    let elems: Vec<_> = repository.split("/").collect();
    if elems.len() != 2 {
        panic!("incorrect repository path");
    }
    let user = &elems[0];
    let project = &elems[1];

    let config = Config::load().unwrap();
    let app: App = App::new(config).unwrap();
    let repo_path = app.resolve_repository_path(user, project).expect(
        "failed to resolve repository path",
    );

    let err = Command::new(action).arg(repo_path.to_str().unwrap()).exec();
    panic!("failed to exec: {:?}", err)
}

fn show(_m: &clap::ArgMatches) {
    let config = Config::load().unwrap();
    let app = App::new(config).unwrap();
    let conn = app.get_db_conn().unwrap();

    let keys: Vec<PublicKey> = public_keys::table.load(&*conn).unwrap();
    for key in keys {
        println!("command=\"/opt/gallium/bin/pubkey access\" {}", key.key);
    }
}
