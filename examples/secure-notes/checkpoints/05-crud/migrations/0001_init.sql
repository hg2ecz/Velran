CREATE TABLE notes (
    id INTEGER PRIMARY KEY,
    ownerUsername TEXT NOT NULL,
    title TEXT NOT NULL CHECK (length(title) BETWEEN 1 AND 120),
    body TEXT NOT NULL CHECK (length(body) BETWEEN 1 AND 4000),
    status TEXT NOT NULL CHECK (status IN ('Draft', 'Published')),
    createdAt TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updatedAt TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX notes_owner_id_idx ON notes(ownerUsername, id DESC);
