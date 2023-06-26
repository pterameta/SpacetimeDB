use crate::api::{from_json_seed, ClientApi, Connection, StmtResultJson};
use anyhow::Context;
use clap::Arg;
use clap::ArgAction;
use clap::ArgMatches;
use reqwest::RequestBuilder;
use spacetimedb_lib::de::serde::SeedWrapper;
use spacetimedb_lib::name::{is_address, DnsLookupResponse};
use spacetimedb_lib::sats::satn;
use spacetimedb_lib::sats::Typespace;
use tabled::builder::Builder;
use tabled::Style;

use crate::config::Config;
use crate::util::get_auth_header;
use crate::util::spacetime_dns;

pub fn cli() -> clap::Command {
    clap::Command::new("sql")
        .about("Runs a SQL query on the database.")
        .arg(
            Arg::new("database")
                .required(true)
                .help("The domain or address of the database you would like to query"),
        )
        .arg(
            Arg::new("query")
                .required(true)
                .help("The SQL query to execute"),
        )
        .arg(
            Arg::new("as_identity")
                .long("as-identity")
                .short('i')
                .conflicts_with("anon_identity")
                .help("The identity to use for querying the database")
                .long_help("The identity to use for querying the database. If no identity is provided, the default one will be used."),
        )
        .arg(
            Arg::new("anon_identity")
                .long("anon-identity")
                .short('a')
                .conflicts_with("as_identity")
                .action(ArgAction::SetTrue)
                .help("If this flag is present, no identity will be provided when querying the database")
        )
}

pub(crate) async fn parse_req(mut config: Config, args: &ArgMatches) -> Result<Connection, anyhow::Error> {
    let database = args.get_one::<String>("database").unwrap();

    let as_identity = args.get_one::<String>("as_identity");
    let anon_identity = args.get_flag("anon_identity");

    let auth_header = get_auth_header(&mut config, anon_identity, as_identity.map(|x| x.as_str()))
        .await
        .map(|x| x.0);

    let address = if is_address(database.as_str()) {
        database.clone()
    } else {
        match spacetime_dns(&config, database).await? {
            DnsLookupResponse::Success { domain: _, address } => address,
            DnsLookupResponse::Failure { domain } => {
                return Err(anyhow::anyhow!("The dns resolution of {} failed.", domain));
            }
        }
    };

    let con = Connection {
        host: config.get_host_url(),
        address,
        database: database.to_string(),
        auth_header,
    };

    Ok(con)
}

pub(crate) async fn run_sql(builder: RequestBuilder, sql: &str) -> Result<(), anyhow::Error> {
    let res = builder.body(sql.to_owned()).send().await?;
    let res = res.error_for_status()?;

    let body = res.bytes().await.unwrap();
    let json = String::from_utf8(body.to_vec()).unwrap();

    let stmt_result_json: Vec<StmtResultJson> = serde_json::from_str(&json).unwrap();

    let stmt_result = stmt_result_json.first().context("Invalid sql query.")?;
    let StmtResultJson { schema, rows } = &stmt_result;

    let mut builder = Builder::default();
    builder.set_columns(
        schema
            .elements
            .iter()
            .enumerate()
            .map(|(i, e)| e.name.clone().unwrap_or_else(|| format!("column {i}"))),
    );

    let typespace = Typespace::default();
    let ty = typespace.with_type(schema);
    for row in rows {
        let row = from_json_seed(row.get(), SeedWrapper(ty))?;
        builder.add_record(
            row.elements
                .iter()
                .zip(&schema.elements)
                .map(|(v, e)| satn::PsqlWrapper(ty.with(&e.algebraic_type).with_value(v))),
        );
    }

    let table = builder.build().with(Style::psql());

    println!("{}", table);

    Ok(())
}

pub async fn exec(config: Config, args: &ArgMatches) -> Result<(), anyhow::Error> {
    let query = args.get_one::<String>("query").unwrap();

    let con = parse_req(config, args).await?;
    let api = ClientApi::new(con);

    run_sql(api.sql(), query).await?;

    Ok(())
}