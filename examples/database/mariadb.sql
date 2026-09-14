CREATE TABLE IF NOT EXISTS products (
    id BIGINT NOT NULL AUTO_INCREMENT PRIMARY KEY,
    name TEXT NOT NULL,
    price BIGINT NOT NULL,
    CHECK (price >= 0)
);
INSERT INTO products(name, price) VALUES ('Keyboard', 19900);
