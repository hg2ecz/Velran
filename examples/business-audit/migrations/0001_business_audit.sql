CREATE TABLE _velran_business_audit (
    event_id VARCHAR(36) PRIMARY KEY,
    occurred_at VARCHAR(35) NOT NULL,
    request_id VARCHAR(128) NOT NULL,
    actor VARCHAR(255) NOT NULL,
    source_action VARCHAR(128) NOT NULL,
    object_type VARCHAR(64) NOT NULL,
    object_id VARCHAR(255) NOT NULL,
    action VARCHAR(64) NOT NULL,
    previous_value VARCHAR(255) NOT NULL,
    new_value VARCHAR(255) NOT NULL
);

CREATE INDEX _velran_business_audit_object_idx
    ON _velran_business_audit(object_type, object_id, occurred_at);

CREATE INDEX _velran_business_audit_actor_idx
    ON _velran_business_audit(actor, occurred_at);
