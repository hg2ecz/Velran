CREATE TABLE IF NOT EXISTS products (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL,
    price INTEGER NOT NULL CHECK (price >= 0)
);
INSERT INTO products(name, price) VALUES ('Keyboard', 19900), ('Mouse', 9900);
