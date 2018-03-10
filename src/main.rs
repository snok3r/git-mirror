/*
 * Copyright (c) 2017 Pascal Bach
 *
 * SPDX-License-Identifier:     MIT
 */

use std::env;
use std::cmp;

// Used for error and debug logging
#[macro_use]
extern crate log;
extern crate stderrlog;

// Used to do command line parsing
#[macro_use]
extern crate clap;
use clap::{App, Arg};

// Load the real functionality
extern crate git_mirror;
use git_mirror::do_mirror;
use git_mirror::MirrorOptions;
use git_mirror::provider::{GitHub, GitLab, Provider};

use std::process::exit;

arg_enum!{
    #[derive(Debug)]
    enum Providers {
      GitLab,
      GitHub
    }
}

fn main() {
    let m = App::new(crate_name!())
        .author(crate_authors!())
        .version(crate_version!())
        .arg(
            Arg::with_name("url")
                .short("u")
                .long("url")
                .help("URL of the instance to get repositories from")
                .default_value_if("provider", Some("GitLab"), "https://gitlab.com")
                .default_value_if("provider", Some("GitHub"), "https://api.github.com"),
        )
        .arg(
            Arg::with_name("group")
                .short("g")
                .long("group")
                .help("Name of the group to check for repositories to sync")
                .takes_value(true)
                .required(true),
        )
        .arg(
            Arg::with_name("mirror-dir")
                .short("m")
                .long("mirror-dir")
                .help("Directory where the local clones are stored")
                .takes_value(true)
                .default_value("./mirror-dir"),
        )
        .arg(
            Arg::with_name("v")
                .short("v")
                .multiple(true)
                .help("Verbosity level"),
        )
        .arg(
            Arg::with_name("http")
                .long("https")
                .help("Use http(s) instead of SSH to sync the GitLab repository"),
        )
        .arg(
            Arg::with_name("dry-run")
                .long("dry-run")
                .help("Only print what to do without actually running any git commands."),
        )
        .arg(
            Arg::with_name("fetch-only")
                .long("fetch-only")
                .help("Fetch the changes from remote and don't push local ones to remote."),
        )
        .arg(
            Arg::with_name("worker-count")
                .short("c")
                .long("worker-count")
                .help("Number of concurrent mirror jobs")
                .default_value("1"),
        )
        .arg(
            Arg::with_name("provider")
                .short("p")
                .long("provider")
                .help("Provider to use for fetching repositories")
                .takes_value(true)
                .possible_values(&Providers::variants())
                .default_value("GitLab"),
        )
        .arg(
            Arg::with_name("metrics-file")
                .long("metrics-file")
                .help(
                    "Location where to store metrics for consumption by \
                     Prometheus nodeexporter's text file colloctor.",
                )
                .takes_value(true),
        )
        .arg(
            Arg::with_name("git-executable")
                .long("git-executable")
                .help("Git executable to use.")
                .takes_value(true)
                .default_value("git"),
        )
        .after_help(
            "ENVIRONMENT:\n    GITLAB_PRIVATE_TOKEN    \
             Private token or Personal access token to access the GitLab API",
        )
        .get_matches();

    stderrlog::new()
        .module(module_path!())
        .timestamp(stderrlog::Timestamp::Second)
        .verbosity(cmp::min(m.occurrences_of("v") as usize, 4))
        .init()
        .unwrap();

    let gitlab_private_token = env::var("GITLAB_PRIVATE_TOKEN").ok();

    // Make sense of the arguments
    let mirror_dir = value_t_or_exit!(m.value_of("mirror-dir"), String);
    debug!("Using mirror directory: {}", mirror_dir);
    let gitlab_url = value_t_or_exit!(m.value_of("url"), String);
    debug!("Using gitlab url: {}", gitlab_url);
    let mirror_group = value_t_or_exit!(m.value_of("group"), String);
    debug!("Using group: {}", mirror_group);
    let use_http = m.is_present("http");
    debug!("Using http enabled: {}", use_http);
    let dry_run = m.is_present("dry-run");
    debug!("Dry run: {}", dry_run);
    let fetch_only = m.is_present("fetch-only");
    debug!("Fetch only is on: {}", fetch_only);
    let worker_count = value_t_or_exit!(m.value_of("worker-count"), usize);
    debug!("Worker count: {}", worker_count);
    let metrics_file = value_t!(m.value_of("metrics-file"), String).ok();
    debug!("Metrics file: {:?}", metrics_file);
    let git_executable = value_t_or_exit!(m.value_of("git-executable"), String);
    debug!("Git executable: {:?}", git_executable);

    let provider: Box<Provider> = match value_t_or_exit!(m.value_of("provider"), Providers) {
        Providers::GitLab => Box::new(GitLab {
            url: gitlab_url.to_owned(),
            group: mirror_group.to_owned(),
            use_http,
            private_token: gitlab_private_token,
            recursive: true,
        }),
        Providers::GitHub => Box::new(GitHub {
            url: gitlab_url.to_owned(),
            org: mirror_group.to_owned(),
            use_http,
            private_token: gitlab_private_token,
            useragent: format!("{}/{}", crate_name!(), crate_version!()),
        }),
    };

    let opts = MirrorOptions {
        dry_run,
        fetch_only,
        worker_count,
        metrics_file,
        git_executable,
    };

    match do_mirror(&provider, &mirror_dir, opts) {
        Ok(_) => {
            info!("All done");
        }
        Err(e) => {
            error!("Error occured: {}", e);
            exit(2); // TODO: Return code in erro
        }
    };
}
