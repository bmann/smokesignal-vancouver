CREATE TABLE events (
    aturi VARCHAR(1024) PRIMARY KEY,
    cid VARCHAR(256) NOT NULL,
    did VARCHAR(256) NOT NULL,
    lexicon VARCHAR(1024) NOT NULL,
    record JSON NOT NULL,
    name VARCHAR(1024) NOT NULL,
    updated_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW ()
);
CREATE INDEX idx_events_did ON events (did);
CREATE TABLE rsvps (
    aturi VARCHAR(1024) PRIMARY KEY,
    cid VARCHAR(256) NOT NULL,
    did VARCHAR(256) NOT NULL,
    lexicon VARCHAR(1024) NOT NULL,
    record JSON NOT NULL,
    event_aturi VARCHAR(1024) NOT NULL,
    event_cid VARCHAR(256) NOT NULL,
    status VARCHAR(1024) NOT NULL DEFAULT 'interested',
    updated_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW ()
);
CREATE INDEX idx_rsvps_did ON rsvps (did);
CREATE INDEX idx_rsvps_event ON rsvps (event_aturi, event_cid);