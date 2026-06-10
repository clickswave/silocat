-- Threaded replies on support tickets (user <-> admin conversation).
CREATE TABLE support_ticket_replies (
    id          TEXT PRIMARY KEY DEFAULT gen_random_uuid()::TEXT,
    ticket_id   TEXT        NOT NULL REFERENCES support_tickets(id) ON DELETE CASCADE,
    author_role TEXT        NOT NULL,            -- 'user' | 'admin'
    author_name TEXT        NOT NULL DEFAULT '',
    body        TEXT        NOT NULL,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_support_replies_ticket ON support_ticket_replies (ticket_id, created_at);
