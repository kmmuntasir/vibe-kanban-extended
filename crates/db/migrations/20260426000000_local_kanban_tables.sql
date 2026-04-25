-- Local Kanban tables: organizations, project statuses, issues, tags, etc.
-- Adapted from remote PostgreSQL schema (20260112000000_remote-projects.sql)
-- for local SQLite use. Skips multi-user features (followers, notifications).

-- 1. ORGANIZATIONS
CREATE TABLE organizations (
    id           BLOB PRIMARY KEY,
    name         TEXT NOT NULL,
    slug         TEXT NOT NULL DEFAULT 'local',
    is_personal  INTEGER NOT NULL DEFAULT 1,
    issue_prefix TEXT NOT NULL DEFAULT 'ISS',
    created_at   TEXT NOT NULL DEFAULT (datetime('now', 'subsec')),
    updated_at   TEXT NOT NULL DEFAULT (datetime('now', 'subsec'))
);

-- 2. ADD KANBAN COLUMNS TO PROJECTS
ALTER TABLE projects ADD COLUMN organization_id BLOB REFERENCES organizations(id);
ALTER TABLE projects ADD COLUMN color TEXT NOT NULL DEFAULT '0 0% 0%';
ALTER TABLE projects ADD COLUMN issue_counter INTEGER NOT NULL DEFAULT 0;

-- 3. PROJECT STATUSES
CREATE TABLE project_statuses (
    id         BLOB PRIMARY KEY,
    project_id BLOB NOT NULL,
    name       TEXT NOT NULL,
    color      TEXT NOT NULL,
    sort_order INTEGER NOT NULL DEFAULT 0,
    hidden     INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL DEFAULT (datetime('now', 'subsec')),
    FOREIGN KEY (project_id) REFERENCES projects(id) ON DELETE CASCADE
);

CREATE INDEX idx_project_statuses_project_id ON project_statuses(project_id);

-- 4. ISSUES
-- issue_number and simple_id generated in Rust layer (no triggers in SQLite).
CREATE TABLE issues (
    id                BLOB PRIMARY KEY,
    project_id        BLOB NOT NULL,
    issue_number      INTEGER NOT NULL,
    simple_id         TEXT NOT NULL,
    status_id         BLOB NOT NULL,
    title             TEXT NOT NULL,
    description       TEXT,
    priority          TEXT CHECK (priority IN ('urgent', 'high', 'medium', 'low')),
    start_date        TEXT,
    target_date       TEXT,
    completed_at      TEXT,
    sort_order        REAL NOT NULL DEFAULT 0,
    parent_issue_id   BLOB REFERENCES issues(id) ON DELETE SET NULL,
    parent_issue_sort_order REAL,
    extension_metadata TEXT NOT NULL DEFAULT '{}',
    created_at        TEXT NOT NULL DEFAULT (datetime('now', 'subsec')),
    updated_at        TEXT NOT NULL DEFAULT (datetime('now', 'subsec')),
    FOREIGN KEY (project_id) REFERENCES projects(id) ON DELETE CASCADE,
    FOREIGN KEY (status_id) REFERENCES project_statuses(id),
    UNIQUE (project_id, issue_number)
);

CREATE INDEX idx_issues_project_id ON issues(project_id);
CREATE INDEX idx_issues_status_id ON issues(status_id);
CREATE INDEX idx_issues_parent_issue_id ON issues(parent_issue_id);
CREATE INDEX idx_issues_simple_id ON issues(simple_id);

-- 5. ISSUE ASSIGNEES
CREATE TABLE issue_assignees (
    id          BLOB PRIMARY KEY,
    issue_id    BLOB NOT NULL,
    user_id     BLOB NOT NULL,
    assigned_at TEXT NOT NULL DEFAULT (datetime('now', 'subsec')),
    FOREIGN KEY (issue_id) REFERENCES issues(id) ON DELETE CASCADE,
    UNIQUE (issue_id, user_id)
);

-- 6. ISSUE RELATIONSHIPS
CREATE TABLE issue_relationships (
    id                BLOB PRIMARY KEY,
    issue_id          BLOB NOT NULL,
    related_issue_id  BLOB NOT NULL,
    relationship_type TEXT NOT NULL CHECK (relationship_type IN ('blocking', 'related', 'has_duplicate')),
    created_at        TEXT NOT NULL DEFAULT (datetime('now', 'subsec')),
    FOREIGN KEY (issue_id) REFERENCES issues(id) ON DELETE CASCADE,
    FOREIGN KEY (related_issue_id) REFERENCES issues(id) ON DELETE CASCADE,
    UNIQUE (issue_id, related_issue_id, relationship_type),
    CHECK (issue_id != related_issue_id)
);

-- 7. TAGS
CREATE TABLE tags (
    id         BLOB PRIMARY KEY,
    project_id BLOB NOT NULL,
    name       TEXT NOT NULL,
    color      TEXT NOT NULL,
    FOREIGN KEY (project_id) REFERENCES projects(id) ON DELETE CASCADE,
    UNIQUE (project_id, name)
);

CREATE TABLE issue_tags (
    id       BLOB PRIMARY KEY,
    issue_id BLOB NOT NULL,
    tag_id   BLOB NOT NULL,
    FOREIGN KEY (issue_id) REFERENCES issues(id) ON DELETE CASCADE,
    FOREIGN KEY (tag_id) REFERENCES tags(id) ON DELETE CASCADE,
    UNIQUE (issue_id, tag_id)
);

-- 8. ISSUE COMMENTS
CREATE TABLE issue_comments (
    id         BLOB PRIMARY KEY,
    issue_id   BLOB NOT NULL,
    author_id  BLOB,
    parent_id  BLOB REFERENCES issue_comments(id) ON DELETE SET NULL,
    message    TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT (datetime('now', 'subsec')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now', 'subsec')),
    FOREIGN KEY (issue_id) REFERENCES issues(id) ON DELETE CASCADE
);

CREATE INDEX idx_issue_comments_issue_id ON issue_comments(issue_id);
CREATE INDEX idx_issue_comments_parent_id ON issue_comments(parent_id);
