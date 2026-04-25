use sqlx::SqlitePool;
use uuid::Uuid;

use crate::models::organization::Organization;
use crate::models::project::{CreateKanbanProject, KanbanProject};
use crate::models::project_status::{CreateProjectStatus, ProjectStatus};
use crate::models::kanban_tag::{CreateKanbanTag, KanbanTag};

const DEFAULT_ORG_NAME: &str = "My Workspace";
const DEFAULT_ORG_SLUG: &str = "local";
const DEFAULT_PROJECT_NAME: &str = "Initial Project";
const DEFAULT_PROJECT_COLOR: &str = "217 91% 60%";

const DEFAULT_STATUSES: [(&str, &str, i32, bool); 6] = [
    ("Backlog", "220 9% 46%", 0, true),
    ("To do", "217 91% 60%", 1, false),
    ("In progress", "38 92% 50%", 2, false),
    ("In review", "258 90% 66%", 3, false),
    ("Done", "142 71% 45%", 4, false),
    ("Cancelled", "0 84% 60%", 5, true),
];

const DEFAULT_TAGS: [(&str, &str); 4] = [
    ("bug", "355 65% 53%"),
    ("feature", "124 82% 30%"),
    ("documentation", "205 100% 40%"),
    ("enhancement", "181 72% 78%"),
];

/// Ensures a default organization and project exist for local Kanban.
/// Called on first boot or when no organizations exist.
pub async fn ensure_default_organization_and_project(
    pool: &SqlitePool,
) -> Result<(Organization, KanbanProject), sqlx::Error> {
    // Check if any org already exists
    let existing = Organization::find_all(pool).await?;
    if let Some(org) = existing.first() {
        // Org exists — find or create default project
        let projects = KanbanProject::find_by_organization(pool, org.id).await?;
        if let Some(project) = projects.first() {
            return Ok((org.clone(), project.clone()));
        }

        // Org exists but no project — create default
        let project = create_default_project(pool, org.id).await?;
        return Ok((org.clone(), project));
    }

    // No org exists — create everything
    let org_id = Uuid::new_v4();
    let org = Organization::create(pool, org_id, DEFAULT_ORG_NAME, DEFAULT_ORG_SLUG, "ISS").await?;
    let project = create_default_project(pool, org.id).await?;
    Ok((org, project))
}

async fn create_default_project(
    pool: &SqlitePool,
    org_id: Uuid,
) -> Result<KanbanProject, sqlx::Error> {
    let project = KanbanProject::create(
        pool,
        &CreateKanbanProject {
            id: None,
            organization_id: org_id,
            name: DEFAULT_PROJECT_NAME.to_string(),
            color: DEFAULT_PROJECT_COLOR.to_string(),
        },
    )
    .await?;

    // Create default statuses
    for (name, color, sort_order, hidden) in DEFAULT_STATUSES {
        let _ = ProjectStatus::create(
            pool,
            &CreateProjectStatus {
                id: None,
                project_id: project.id,
                name: name.to_string(),
                color: color.to_string(),
                sort_order,
                hidden,
            },
        )
        .await;
    }

    // Create default tags
    for (name, color) in DEFAULT_TAGS {
        let _ = KanbanTag::create(
            pool,
            &CreateKanbanTag {
                id: None,
                project_id: project.id,
                name: name.to_string(),
                color: color.to_string(),
            },
        )
        .await;
    }

    Ok(project)
}
