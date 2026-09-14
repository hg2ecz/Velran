CREATE TABLE markdown_documents (
    id BIGINT PRIMARY KEY,
    title TEXT NOT NULL,
    body TEXT NOT NULL
);

INSERT INTO markdown_documents (id, title, body) VALUES (
    1,
    'Velran Markdown example',
    E'# Hello from SQL\n\nThis **Markdown** is stored as source text and rendered on the server.\n\n[Velran root](/)'
);
