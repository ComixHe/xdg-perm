use std::{collections::HashMap, error::Error};

use clap::{Args, Parser, Subcommand};
use comfy_table::Table;
use zbus::{proxy, zvariant::OwnedValue, Connection};

// Cli struct

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
#[command(propagate_version = true)]
struct Cli {
    #[command(subcommand)]
    command: Subcommands,
}

#[derive(Subcommand, Debug)]
enum Subcommands {
    /// Delete Permissions
    Delete(DeleteArgs),

    /// Get Permissions
    Get(GetArgs),

    /// List Permissions
    List(ListArgs),

    /// Lookup Permissions
    Lookup(LookupArgs),
    // /// Set Permissions
    // Set(SetArgs),
}

#[derive(Args, Debug)]
struct LookupArgs {
    /// The name of the table to use
    table: String,

    /// The resource ID to modify
    id: String,
}

#[derive(Args, Debug)]
struct ListArgs {
    /// The name of the table to use
    table: String,
}

#[derive(Args, Debug)]
struct GetArgs {
    /// The name of the table to use
    table: String,

    /// The resource ID to modify
    id: String,

    /// Name of the application
    app: String,
}

#[derive(Args, Debug)]
struct DeleteArgs {
    /// The name of the table to use
    table: String,

    /// The resource ID to modify
    id: String,

    /// Name of the application
    app: Option<String>,
}

//TODO: support setting permissions

// custom DBus type

type LookupResponse = (HashMap<String, Vec<String>>, OwnedValue);

#[proxy(
    interface = "org.freedesktop.impl.portal.PermissionStore",
    default_service = "org.freedesktop.impl.portal.PermissionStore",
    default_path = "/org/freedesktop/impl/portal/PermissionStore",
    gen_async = true
)]
trait PermissionStore {
    #[zbus(property(emits_changed_signal = "const"), name = "version")]
    fn version(&self) -> zbus::Result<u32>;

    fn delete(&self, table: &str, id: &str) -> zbus::Result<()>;
    fn delete_permission(&self, table: &str, id: &str, app: &str) -> zbus::Result<()>;
    fn get_permission(&self, table: &str, id: &str, app: &str) -> zbus::Result<Vec<String>>;
    fn list(&self, table: &str) -> zbus::Result<Vec<String>>;
    fn lookup(&self, table: &str, id: &str) -> zbus::Result<LookupResponse>;
    fn set(
        &self,
        table: &str,
        create: bool,
        id: &str,
        app_permissions: Vec<HashMap<String, Vec<String>>>,
        data: OwnedValue,
    ) -> zbus::Result<()>;
    fn set_permission(
        &self,
        table: &str,
        create: bool,
        id: &str,
        app: &str,
        permissions: Vec<String>,
    ) -> zbus::Result<()>;
    fn set_value(&self, create: bool, id: &str, data: OwnedValue) -> zbus::Result<()>;
}

// main impl

const PERMISSION_STORE_SPEC_VER: u32 = 2;

async fn delete_permission(
    proxy: &PermissionStoreProxy<'_>,
    args: &DeleteArgs,
) -> zbus::Result<()> {
    match &args.app {
        Some(app) => proxy.delete_permission(&args.table, &args.id, app).await,
        None => proxy.delete(&args.table, &args.id).await,
    }
}

fn print_lookup_response(response: &LookupResponse) {
    let mut table = Table::new();
    table.set_header(vec!["AppID", "Permissions"]);

    for (app_id, allowed) in response.0.iter() {
        table.add_row(vec![app_id, &allowed.join(",")]);
    }

    println!("{table}");
    println!("associated data:\n{:?}", response.1);
}

fn print_list_response(response: &[String]) {
    let mut table = Table::new();
    table.set_header(vec!["Resource ID"]);

    for id in response.iter() {
        table.add_row(vec![id]);
    }

    println!("{table}");
}

fn print_get_permission_response(response: &[String]) {
    let mut table = Table::new();
    table.set_header(vec!["Permission"]);

    for permission in response.iter() {
        table.add_row(vec![permission]);
    }

    println!("{table}");
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let connection = Connection::session().await?;
    let proxy = PermissionStoreProxy::new(&connection).await?;

    let server_version = proxy.version().await?;
    if server_version != PERMISSION_STORE_SPEC_VER {
        return Err(format!(
            "Server version {server_version} does not match expected version {PERMISSION_STORE_SPEC_VER}")
        .into());
    }

    let cli = Cli::parse();
    match &cli.command {
        Subcommands::Delete(args) => match delete_permission(&proxy, args).await {
            Ok(_) => println!("Permissions deleted successfully"),
            Err(e) => return Err(e.into()),
        },
        Subcommands::Get(GetArgs { table, id, app }) => {
            let permissions = proxy.get_permission(table, id, app).await?;
            print_get_permission_response(&permissions);
        }
        Subcommands::List(ListArgs { table }) => {
            let ids = proxy.list(table).await?;
            print_list_response(&ids);
        }
        Subcommands::Lookup(LookupArgs { table, id }) => {
            let result = proxy.lookup(table, id).await?;
            print_lookup_response(&result);
        }
    };

    Ok(())
}
