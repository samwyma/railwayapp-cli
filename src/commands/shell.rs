use std::collections::BTreeMap;

use crate::consts::SERVICE_NOT_FOUND;
use crate::controllers::variables::get_service_variables;

use super::*;

/// Open a subshell with Railway variables available
#[derive(Parser)]
pub struct Args {
    /// Service to pull variables from (defaults to linked service)
    #[clap(short, long)]
    service: Option<String>,
}

pub async fn command(args: Args, _json: bool) -> Result<()> {
    let configs = Configs::new()?;
    let client = GQLClient::new_authorized(&configs)?;
    let linked_project = configs.get_linked_project().await?;

    let vars = queries::project::Variables {
        id: linked_project.project.to_owned(),
    };

    let res = post_graphql::<queries::Project, _>(&client, configs.get_backboard(), vars).await?;

    let body = res.data.context("Failed to retrieve response body")?;
    let mut all_variables = BTreeMap::<String, String>::new();
    all_variables.insert("IN_RAILWAY_SHELL".to_owned(), "true".to_owned());

    if let Some(service) = args.service {
        let service_id = body
            .project
            .services
            .edges
            .iter()
            .find(|s| s.node.name == service || s.node.id == service)
            .context(SERVICE_NOT_FOUND)?;

        let service_variables = get_service_variables(
            &client,
            &configs,
            linked_project.project.clone(),
            linked_project.environment,
            service_id.node.id.clone(),
        )
        .await?;

        all_variables.extend(service_variables);
    } else if let Some(service) = linked_project.service {
        let service_variables = get_service_variables(
            &client,
            &configs,
            linked_project.project.clone(),
            linked_project.environment,
            service.clone(),
        )
        .await?;

        all_variables.extend(service_variables);
    } else {
        eprintln!("No service linked, skipping service variables");
    }

    let shell = std::env::var("SHELL").unwrap_or(match std::env::consts::OS {
        "windows" => "cmd".to_string(),
        _ => "sh".to_string(),
    });

    println!("Entering subshell with Railway variables available. Type 'exit' to exit.");

    tokio::process::Command::new(shell)
        .envs(all_variables)
        .spawn()
        .context("Failed to spawn command")?
        .wait()
        .await
        .context("Failed to wait for command")?;
    println!("Exited subshell, Railway variables no longer available.");
    Ok(())
}
