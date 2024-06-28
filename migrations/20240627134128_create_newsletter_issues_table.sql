CREATE TABLE newsletter_issues (
    newsletter_issue_id uuid NOT NULL,
    title TEXT NOT NULL,
    text_content TEXT NOT NULL,
    html_content TEXT NOT NULL,
    published_at TIMESTAMPTZ NOT NULL,
    PRIMARY KEY (newsletter_issue_id)
);
