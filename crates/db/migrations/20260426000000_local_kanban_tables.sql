PRAGMA foreign_keys = ON;

-- 1. ORGANIZATIONS
CREATE TABLE organizations (
    id            BLOB PRIMARY KEY,
    name          TEXT NOT NULL,
    slug          TEXT NOT NULL UNIQUE,
    is_personal   INTEGER NOT NULL DEFAULT 0,
    issue_prefix  TEXT NOT NULL DEFAULT 'VK',
    created_at    TEXT NOT NULL DEFAULT (datetime('now', 'subsec')),
    updated_at    TEXT NOT NULL DEFAULT (datetime('now', 'subsec'))
);

-- 2. ALTER EXISTING PROJECTS TABLE
ALTER TABLE projects ADD COLUMN color TEXT NOT NULL DEFAULT '0 0% 0%';
ALTER TABLE projects ADD COLUMN issue_counter INTEGER NOT NULL DEFAULT 0;
ALTER TABLE projects ADD COLUMN organization_id BLOB REFERENCES organizations(id) ON DELETE SET NULL;

-- 3. PROJECT STATUSES
CREATE TABLE project_statuses (
    id          BLOB PRIMARY KEY,
    project_id  BLOB NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    name        TEXT NOT NULL,
    color       TEXT NOT NULL,
    sort_order  INTEGER NOT NULL DEFAULT 0,
    hidden      INTEGER NOT NULL DEFAULT 0,
    created_at  TEXT NOT NULL DEFAULT (datetime('now', 'subsec'))
);

-- 4. ISSUES
CREATE TABLE issues (
    id                        BLOB PRIMARY KEY,
    project_id                BLOB NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    issue_number              INTEGER NOT NULL,
    simple_id                 TEXT NOT NULL UNIQUE,
    status_id                 BLOB REFERENCES project_statuses(id) ON DELETE SET NULL,
    title                     TEXT NOT NULL,
    description               TEXT,
    priority                  TEXT NOT NULL DEFAULT 'medium' CHECK(priority IN ('urgent', 'high', 'medium', 'low')),
    start_date                TEXT,
    target_date               TEXT,
    completed_at              TEXT,
    sort_order                INTEGER NOT NULL DEFAULT 0,
    parent_issue_id           BLOB REFERENCES issues(id) ON DELETE SET NULL,
    parent_issue_sort_order   REAL,
    extension_metadata        TEXT NOT NULL DEFAULT '{}',
    creator_user_id           BLOB,
    created_at                TEXT NOT NULL DEFAULT (datetime('now', 'subsec')),
    updated_at                TEXT NOT NULL DEFAULT (datetime('now', 'subsec')),
    UNIQUE (project_id, issue_number)
);

-- 5. ISSUE ASSIGNEES
CREATE TABLE issue_assignees (
    id          BLOB PRIMARY KEY,
    issue_id    BLOB NOT NULL REFERENCES issues(id) ON DELETE CASCADE,
    user_id     BLOB NOT NULL,
    assigned_at TEXT NOT NULL DEFAULT (datetime('now', 'subsec')),
    UNIQUE (issue_id, user_id)
);

-- 6. ISSUE RELATIONSHIPS
CREATE TABLE issue_relationships (
    id                BLOB PRIMARY KEY,
    issue_id          BLOB NOT NULL REFERENCES issues(id) ON DELETE CASCADE,
    related_issue_id  BLOB NOT NULL REFERENCES issues(id) ON DELETE CASCADE,
    relationship_type TEXT NOT NULL CHECK(relationship_type IN ('blocking', 'related', 'has_duplicate')),
    created_at        TEXT NOT NULL DEFAULT (datetime('now', 'subsec')),
    UNIQUE (issue_id, related_issue_id, relationship_type),
    CHECK (issue_id != related_issue_id)
);

-- 7. KANBAN TAGS (named kanban_tags to avoid collision with existing tags table)
CREATE TABLE kanban_tags (
    id          BLOB PRIMARY KEY,
    project_id  BLOB NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    name        TEXT NOT NULL,
    color       TEXT NOT NULL,
    created_at  TEXT NOT NULL DEFAULT (datetime('now', 'subsec')),
    updated_at  TEXT NOT NULL DEFAULT (datetime('now', 'subsec'))
);

-- 8. ISSUE TAGS
CREATE TABLE issue_tags (
    id       BLOB PRIMARY KEY,
    issue_id BLOB NOT NULL REFERENCES issues(id) ON DELETE CASCADE,
    tag_id   BLOB NOT NULL REFERENCES kanban_tags(id) ON DELETE CASCADE,
    UNIQUE (issue_id, tag_id)
);

-- 9. ISSUE COMMENTS
CREATE TABLE issue_comments (
    id          BLOB PRIMARY KEY,
    issue_id    BLOB NOT NULL REFERENCES issues(id) ON DELETE CASCADE,
    author_id   BLOB,
    parent_id   BLOB REFERENCES issue_comments(id) ON DELETE SET NULL,
    message     TEXT NOT NULL,
    created_at  TEXT NOT NULL DEFAULT (datetime('now', 'subsec')),
    updated_at  TEXT NOT NULL DEFAULT (datetime('now', 'subsec'))
);

-- INDEXES
CREATE INDEX idx_issues_project_id ON issues(project_id);
CREATE INDEX idx_issues_status_id ON issues(status_id);
CREATE INDEX idx_issues_parent_issue_id ON issues(parent_issue_id);
CREATE INDEX idx_issues_simple_id ON issues(simple_id);
CREATE INDEX idx_issue_comments_issue_id ON issue_comments(issue_id);
CREATE INDEX idx_issue_comments_parent_id ON issue_comments(parent_id);
