//! Local-mode fallbacks for the `/api/remote/*` handlers.
//!
//! When the app runs locally (`VK_SHARED_API_BASE` unset), `Deployment::remote_client()`
//! returns `Err(RemoteClientNotConfigured)`. The original cloud-proxy handlers propagate that
//! as HTTP 400, which breaks every MCP tool call (the `vibe-kanban-mcp` binary targets
//! `/api/remote/*`). These helpers serve the same data straight from the local SQLite DB so
//! the MCP — and any other `/api/remote/*` client — keeps working without a remote backend.
//!
//! Each function mirrors the local query logic already used by the `kanban_v1` routes and
//! returns the exact `api_types::*` shapes the cloud client would have produced.

use api_types::{
    CreateIssueAssigneeRequest, CreateIssueRelationshipRequest, CreateIssueRequest,
    CreateIssueTagRequest, Issue, IssueAssignee, IssuePriority, IssueRelationship,
    IssueRelationshipType, IssueSortField, IssueTag, ListIssueAssigneesResponse,
    ListIssueRelationshipsResponse, ListIssueTagsResponse, ListIssuesResponse,
    ListProjectsResponse, ListTagsResponse, MutationResponse, Project, ProjectStatus,
    SearchIssuesRequest, Tag,
};
use chrono::Utc;
use db::models::{
    issue::{
        Issue as DbIssue, IssueSortField as DbSortField, SearchIssuesRequest as DbSearchRequest,
        SortDirection as DbSortDirection,
    },
    issue_assignee::IssueAssignee as DbIssueAssignee,
    issue_relationship::IssueRelationship as DbIssueRelationship,
    issue_tag::IssueTag as DbIssueTag,
    kanban_tag::KanbanTag,
    project::Project as DbProject,
    project_status::ProjectStatus as DbProjectStatus,
};
use sqlx::SqlitePool;
use uuid::Uuid;

use crate::error::ApiError;

fn txid() -> i64 {
    Utc::now().timestamp_millis()
}

// ---------------------------------------------------------------------------
// Issues
// ---------------------------------------------------------------------------

fn to_api_issue(db: DbIssue) -> Issue {
    Issue {
        id: db.id,
        project_id: db.project_id,
        issue_number: db.issue_number,
        simple_id: db.simple_id,
        status_id: db.status_id,
        title: db.title,
        description: db.description,
        priority: db.priority.parse::<IssuePriority>().ok(),
        start_date: db.start_date,
        target_date: db.target_date,
        completed_at: db.completed_at,
        sort_order: db.sort_order as f64,
        parent_issue_id: db.parent_issue_id,
        parent_issue_sort_order: db.parent_issue_sort_order,
        extension_metadata: serde_json::from_str(&db.extension_metadata).unwrap_or_default(),
        creator_user_id: db.creator_user_id,
        created_at: db.created_at,
        updated_at: db.updated_at,
    }
}

fn priority_to_str(p: &Option<Option<IssuePriority>>) -> Option<Option<&str>> {
    p.as_ref().map(|inner| {
        inner.as_ref().map(|p| match p {
            IssuePriority::Urgent => "urgent",
            IssuePriority::High => "high",
            IssuePriority::Medium => "medium",
            IssuePriority::Low => "low",
        })
    })
}

fn to_db_search(req: &SearchIssuesRequest) -> DbSearchRequest {
    DbSearchRequest {
        project_id: req.project_id,
        status_id: req.status_id,
        status_ids: req.status_ids.clone(),
        priority: req.priority.as_ref().map(|p| match p {
            IssuePriority::Urgent => "urgent".to_string(),
            IssuePriority::High => "high".to_string(),
            IssuePriority::Medium => "medium".to_string(),
            IssuePriority::Low => "low".to_string(),
        }),
        parent_issue_id: req.parent_issue_id,
        search: req.search.clone(),
        simple_id: req.simple_id.clone(),
        assignee_user_id: req.assignee_user_id,
        tag_id: req.tag_id,
        tag_ids: req.tag_ids.clone(),
        sort_field: req.sort_field.map(|f| match f {
            IssueSortField::SortOrder => DbSortField::SortOrder,
            IssueSortField::Priority => DbSortField::Priority,
            IssueSortField::CreatedAt => DbSortField::CreatedAt,
            IssueSortField::UpdatedAt => DbSortField::UpdatedAt,
            IssueSortField::Title => DbSortField::Title,
        }),
        sort_direction: req.sort_direction.map(|d| match d {
            api_types::SortDirection::Asc => DbSortDirection::Asc,
            api_types::SortDirection::Desc => DbSortDirection::Desc,
        }),
        limit: req.limit,
        offset: req.offset,
    }
}

pub async fn list_issues(
    pool: &SqlitePool,
    project_id: Uuid,
) -> Result<ListIssuesResponse, ApiError> {
    let issues = DbIssue::find_by_project(pool, project_id).await?;
    let total_count = issues.len();
    Ok(ListIssuesResponse {
        issues: issues.into_iter().map(to_api_issue).collect(),
        total_count,
        limit: total_count,
        offset: 0,
    })
}

pub async fn search_issues(
    pool: &SqlitePool,
    request: &SearchIssuesRequest,
) -> Result<ListIssuesResponse, ApiError> {
    let db_request = to_db_search(request);
    let response = DbIssue::search(pool, &db_request).await?;
    Ok(ListIssuesResponse {
        issues: response.issues.into_iter().map(to_api_issue).collect(),
        total_count: response.total_count,
        limit: response.limit,
        offset: response.offset,
    })
}

pub async fn get_issue(pool: &SqlitePool, issue_id: Uuid) -> Result<Issue, ApiError> {
    let issue = DbIssue::find_by_id(pool, issue_id)
        .await?
        .ok_or_else(|| ApiError::BadRequest("Issue not found".to_string()))?;
    Ok(to_api_issue(issue))
}

pub async fn create_issue(
    pool: &SqlitePool,
    request: &CreateIssueRequest,
) -> Result<MutationResponse<Issue>, ApiError> {
    let priority_str = request.priority.as_ref().map(|p| match p {
        IssuePriority::Urgent => "urgent",
        IssuePriority::High => "high",
        IssuePriority::Medium => "medium",
        IssuePriority::Low => "low",
    });
    let issue = DbIssue::create(
        pool,
        request.id,
        request.project_id,
        request.status_id,
        &request.title,
        request.description.as_deref(),
        priority_str,
        request.start_date,
        request.target_date,
        request.completed_at,
        request.sort_order as i32,
        request.parent_issue_id,
        request.parent_issue_sort_order,
        &request.extension_metadata,
        None,
    )
    .await?;
    Ok(MutationResponse {
        data: to_api_issue(issue),
        txid: txid(),
    })
}

pub async fn update_issue(
    pool: &SqlitePool,
    issue_id: Uuid,
    request: &api_types::UpdateIssueRequest,
) -> Result<MutationResponse<Issue>, ApiError> {
    let issue = DbIssue::update(
        pool,
        issue_id,
        request.status_id,
        request.title.as_deref(),
        request
            .description
            .as_ref()
            .map(|d| d.as_ref().map(|s| s.as_str())),
        priority_to_str(&request.priority),
        request.start_date,
        request.target_date,
        request.completed_at,
        request.sort_order.map(|s| s as i32),
        request.parent_issue_id,
        request.parent_issue_sort_order,
        request.extension_metadata.as_ref(),
    )
    .await?;
    Ok(MutationResponse {
        data: to_api_issue(issue),
        txid: txid(),
    })
}

pub async fn delete_issue(pool: &SqlitePool, issue_id: Uuid) -> Result<(), ApiError> {
    DbIssue::delete(pool, issue_id).await?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Projects
// ---------------------------------------------------------------------------

fn to_api_project(db: DbProject) -> Project {
    Project {
        id: db.id,
        organization_id: db.organization_id.unwrap_or_default(),
        name: db.name,
        color: db.color,
        sort_order: 0,
        created_at: db.created_at,
        updated_at: db.updated_at,
    }
}

pub async fn list_projects(
    pool: &SqlitePool,
    organization_id: Uuid,
) -> Result<ListProjectsResponse, ApiError> {
    let projects = DbProject::find_by_organization(pool, organization_id)
        .await?
        .into_iter()
        .map(to_api_project)
        .collect();
    Ok(ListProjectsResponse { projects })
}

pub async fn get_project(pool: &SqlitePool, project_id: Uuid) -> Result<Project, ApiError> {
    let project = DbProject::find_by_id(pool, project_id)
        .await?
        .ok_or_else(|| ApiError::BadRequest("Project not found".to_string()))?;
    Ok(to_api_project(project))
}

// ---------------------------------------------------------------------------
// Tags
// ---------------------------------------------------------------------------

fn to_api_tag(db: KanbanTag) -> Tag {
    Tag {
        id: db.id,
        project_id: db.project_id,
        name: db.name,
        color: db.color,
    }
}

pub async fn list_tags(pool: &SqlitePool, project_id: Uuid) -> Result<ListTagsResponse, ApiError> {
    let tags = KanbanTag::find_by_project(pool, project_id)
        .await?
        .into_iter()
        .map(to_api_tag)
        .collect();
    Ok(ListTagsResponse { tags })
}

pub async fn get_tag(pool: &SqlitePool, tag_id: Uuid) -> Result<Tag, ApiError> {
    let tag = KanbanTag::find_by_id(pool, tag_id)
        .await?
        .ok_or_else(|| ApiError::BadRequest("Tag not found".to_string()))?;
    Ok(to_api_tag(tag))
}

// ---------------------------------------------------------------------------
// Issue tags
// ---------------------------------------------------------------------------

pub async fn list_issue_tags(
    pool: &SqlitePool,
    issue_id: Uuid,
) -> Result<ListIssueTagsResponse, ApiError> {
    let issue_tags = DbIssueTag::find_by_issue(pool, issue_id)
        .await?
        .into_iter()
        .map(|t| IssueTag {
            id: t.id,
            issue_id: t.issue_id,
            tag_id: t.tag_id,
        })
        .collect();
    Ok(ListIssueTagsResponse { issue_tags })
}

pub async fn get_issue_tag(pool: &SqlitePool, issue_tag_id: Uuid) -> Result<IssueTag, ApiError> {
    let t = DbIssueTag::find_by_id(pool, issue_tag_id)
        .await?
        .ok_or_else(|| ApiError::BadRequest("Issue tag not found".to_string()))?;
    Ok(IssueTag {
        id: t.id,
        issue_id: t.issue_id,
        tag_id: t.tag_id,
    })
}

pub async fn create_issue_tag(
    pool: &SqlitePool,
    request: &CreateIssueTagRequest,
) -> Result<MutationResponse<IssueTag>, ApiError> {
    let t = DbIssueTag::create(pool, request.id, request.issue_id, request.tag_id).await?;
    Ok(MutationResponse {
        data: IssueTag {
            id: t.id,
            issue_id: t.issue_id,
            tag_id: t.tag_id,
        },
        txid: txid(),
    })
}

pub async fn delete_issue_tag(pool: &SqlitePool, issue_tag_id: Uuid) -> Result<(), ApiError> {
    DbIssueTag::delete(pool, issue_tag_id).await?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Issue assignees
// ---------------------------------------------------------------------------

pub async fn list_issue_assignees(
    pool: &SqlitePool,
    issue_id: Uuid,
) -> Result<ListIssueAssigneesResponse, ApiError> {
    let issue_assignees = DbIssueAssignee::find_by_issue(pool, issue_id)
        .await?
        .into_iter()
        .map(|a| IssueAssignee {
            id: a.id,
            issue_id: a.issue_id,
            user_id: a.user_id,
            assigned_at: a.assigned_at,
        })
        .collect();
    Ok(ListIssueAssigneesResponse { issue_assignees })
}

pub async fn get_issue_assignee(
    pool: &SqlitePool,
    issue_assignee_id: Uuid,
) -> Result<IssueAssignee, ApiError> {
    let a = DbIssueAssignee::find_by_id(pool, issue_assignee_id)
        .await?
        .ok_or_else(|| ApiError::BadRequest("Issue assignee not found".to_string()))?;
    Ok(IssueAssignee {
        id: a.id,
        issue_id: a.issue_id,
        user_id: a.user_id,
        assigned_at: a.assigned_at,
    })
}

pub async fn create_issue_assignee(
    pool: &SqlitePool,
    request: &CreateIssueAssigneeRequest,
) -> Result<MutationResponse<IssueAssignee>, ApiError> {
    let a = DbIssueAssignee::create(pool, request.id, request.issue_id, request.user_id).await?;
    Ok(MutationResponse {
        data: IssueAssignee {
            id: a.id,
            issue_id: a.issue_id,
            user_id: a.user_id,
            assigned_at: a.assigned_at,
        },
        txid: txid(),
    })
}

pub async fn delete_issue_assignee(
    pool: &SqlitePool,
    issue_assignee_id: Uuid,
) -> Result<(), ApiError> {
    DbIssueAssignee::delete(pool, issue_assignee_id).await?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Issue relationships
// ---------------------------------------------------------------------------

fn relationship_type_to_str(t: &IssueRelationshipType) -> &'static str {
    match t {
        IssueRelationshipType::Blocking => "blocking",
        IssueRelationshipType::Related => "related",
        IssueRelationshipType::HasDuplicate => "has_duplicate",
    }
}

fn parse_relationship_type(s: &str) -> IssueRelationshipType {
    match s {
        "related" => IssueRelationshipType::Related,
        "has_duplicate" => IssueRelationshipType::HasDuplicate,
        _ => IssueRelationshipType::Blocking,
    }
}

fn to_api_relationship(db: DbIssueRelationship) -> IssueRelationship {
    IssueRelationship {
        id: db.id,
        issue_id: db.issue_id,
        related_issue_id: db.related_issue_id,
        relationship_type: parse_relationship_type(&db.relationship_type),
        created_at: db.created_at,
    }
}

pub async fn list_issue_relationships(
    pool: &SqlitePool,
    issue_id: Uuid,
) -> Result<ListIssueRelationshipsResponse, ApiError> {
    let issue_relationships = DbIssueRelationship::find_by_issue(pool, issue_id)
        .await?
        .into_iter()
        .map(to_api_relationship)
        .collect();
    Ok(ListIssueRelationshipsResponse {
        issue_relationships,
    })
}

pub async fn create_issue_relationship(
    pool: &SqlitePool,
    request: &CreateIssueRelationshipRequest,
) -> Result<MutationResponse<IssueRelationship>, ApiError> {
    let relationship = DbIssueRelationship::create(
        pool,
        request.id,
        request.issue_id,
        request.related_issue_id,
        relationship_type_to_str(&request.relationship_type),
    )
    .await?;
    Ok(MutationResponse {
        data: to_api_relationship(relationship),
        txid: txid(),
    })
}

pub async fn delete_issue_relationship(
    pool: &SqlitePool,
    relationship_id: Uuid,
) -> Result<(), ApiError> {
    DbIssueRelationship::delete(pool, relationship_id).await?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Project statuses
// ---------------------------------------------------------------------------

fn to_api_project_status(db: DbProjectStatus) -> ProjectStatus {
    ProjectStatus {
        id: db.id,
        project_id: db.project_id,
        name: db.name,
        color: db.color,
        sort_order: db.sort_order,
        hidden: db.hidden,
        created_at: db.created_at,
    }
}

pub async fn list_project_statuses(
    pool: &SqlitePool,
    project_id: Uuid,
) -> Result<api_types::ListProjectStatusesResponse, ApiError> {
    let project_statuses = DbProjectStatus::find_by_project(pool, project_id)
        .await?
        .into_iter()
        .map(to_api_project_status)
        .collect();
    Ok(api_types::ListProjectStatusesResponse { project_statuses })
}
