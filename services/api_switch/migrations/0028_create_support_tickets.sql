-- Support/contact tickets submitted from /home/support. Persisted so the admin
-- panel can list/filter them (email delivery is best-effort, not the source of truth).
CREATE TABLE support_tickets (
    id         TEXT PRIMARY KEY DEFAULT gen_random_uuid()::TEXT,
    user_id    TEXT,
    username   TEXT        NOT NULL DEFAULT '',
    email      TEXT        NOT NULL DEFAULT '',
    category   TEXT        NOT NULL DEFAULT 'other',
    subject    TEXT        NOT NULL,
    message    TEXT        NOT NULL,
    is_pro     BOOLEAN     NOT NULL DEFAULT FALSE, -- snapshot of plan at submission
    status     TEXT        NOT NULL DEFAULT 'open', -- 'open' | 'closed'
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_support_tickets_created_at ON support_tickets (created_at DESC);
CREATE INDEX idx_support_tickets_status ON support_tickets (status);
