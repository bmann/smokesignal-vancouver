CREATE TABLE denylist (
    subject varchar(48) PRIMARY KEY,
    reason TEXT NOT NULL,
    updated_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW ()
);
CREATE INDEX idx_denylist_subject ON denylist(subject);