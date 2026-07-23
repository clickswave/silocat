-- Security/compliance (silocat-suggestions.md P0 #10): a public file host that
-- accepts anonymous uploads needs a way for anyone to report an abusive share
-- link (DMCA / illegal content). Reports land here for admin review + takedown.
CREATE TABLE abuse_reports (
    id          TEXT PRIMARY KEY NOT NULL DEFAULT gen_random_uuid()::TEXT,
    share_token TEXT,
    reason      TEXT             NOT NULL,
    details     TEXT,
    reporter_ip TEXT,
    status      TEXT             NOT NULL DEFAULT 'open', -- open | reviewed | actioned | dismissed
    created_at  TIMESTAMPTZ      NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_abuse_reports_status ON abuse_reports (status, created_at DESC);
CREATE INDEX idx_abuse_reports_token  ON abuse_reports (share_token);
