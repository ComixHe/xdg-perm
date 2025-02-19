use clap::{Args, Parser, Subcommand};
use comfy_table::Table;
use std::collections::HashMap;
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

    /// Set Permissions
    Set(SetArgs),
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

#[derive(Args, Debug)]
struct SetArgs {
    /// Whether to create the table if it does not exist
    #[arg(short, long, default_value_t = false)]
    create: bool,

    /// The name of the table to use
    table: String,

    /// The resource ID to modify
    id: String,

    /// The application ID to modify
    app: String,

    /// The permissions to set
    permissions: Vec<String>,
}

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
    fn set_permission(
        &self,
        table: &str,
        create: bool,
        id: &str,
        app: &str,
        permissions: &[String],
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
async fn main() {
    let connection = match Connection::session().await {
        Ok(connection) => connection,
        Err(e) => {
            eprintln!("Failed to connect: {e}");
            return;
        }
    };

    let proxy = match PermissionStoreProxy::new(&connection).await {
        Ok(proxy) => proxy,
        Err(e) => {
            eprintln!("Failed to create proxy: {e}");
            return;
        }
    };

    let server_version = match proxy.version().await {
        Ok(version) => version,
        Err(e) => {
            eprintln!("Failed to get server version: {e}");
            return;
        }
    };

    if server_version != PERMISSION_STORE_SPEC_VER {
        eprintln!("Server version {server_version} does not match expected version {PERMISSION_STORE_SPEC_VER}");
        return;
    }

    let cli = Cli::parse();
    match &cli.command {
        Subcommands::Delete(args) => match delete_permission(&proxy, args).await {
            Ok(_) => println!("Permissions deleted successfully"),
            Err(e) => eprintln!("failed to delete permissions: {e}"),
        },
        Subcommands::Get(GetArgs { table, id, app }) => {
            match proxy.get_permission(table, id, app).await {
                Ok(permissions) => print_get_permission_response(&permissions),
                Err(e) => eprintln!("failed to get permissions: {e}"),
            }
        }
        Subcommands::List(ListArgs { table }) => match proxy.list(table).await {
            Ok(ids) => print_list_response(&ids),
            Err(e) => eprintln!("failed to list permissions: {e}"),
        },
        Subcommands::Lookup(LookupArgs { table, id }) => match proxy.lookup(table, id).await {
            Ok(result) => print_lookup_response(&result),
            Err(e) => eprintln!("failed to lookup permissions: {e}"),
        },
        Subcommands::Set(args) => match proxy
            .set_permission(
                &args.table,
                args.create,
                &args.id,
                &args.app,
                &args.permissions,
            )
            .await
        {
            Ok(_) => println!("Permissions set successfully"),
            Err(e) => eprintln!("failed to set permissions: {e}"),
        },
    };
}
