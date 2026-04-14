-- Blog tables
CREATE TABLE IF NOT EXISTS categories (
    id   SERIAL PRIMARY KEY,
    name TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS posts (
    id          SERIAL PRIMARY KEY,
    title       TEXT NOT NULL,
    body        TEXT NOT NULL DEFAULT '',
    status      TEXT NOT NULL DEFAULT 'draft',
    category_id INTEGER REFERENCES categories(id)
);

CREATE TABLE IF NOT EXISTS tags (
    id   SERIAL PRIMARY KEY,
    name TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS post_tags (
    post_id INTEGER NOT NULL REFERENCES posts(id) ON DELETE CASCADE,
    tag_id  INTEGER NOT NULL REFERENCES tags(id) ON DELETE CASCADE,
    PRIMARY KEY (post_id, tag_id)
);

-- Seed data
INSERT INTO categories (name) VALUES ('Tech'), ('Rust'), ('Web')
    ON CONFLICT DO NOTHING;

INSERT INTO tags (name) VALUES ('tutorial'), ('performance'), ('async'), ('tooling'), ('beginner')
    ON CONFLICT DO NOTHING;

INSERT INTO posts (title, body, status, category_id) VALUES
    ('Hello World',           'My first post.',                     'published', 1),
    ('Why Rust?',             'Rust is fast and safe.',             'published', 2),
    ('Building Admins',       'axum-admin makes it easy.',          'draft',     3),
    ('Async in Rust',         'Tokio makes async simple.',          'published', 2),
    ('Error Handling',        'Thiserror and anyhow.',              'published', 2),
    ('Axum Basics',           'Routing and extractors.',            'published', 1),
    ('SeaORM Guide',          'Querying with SeaORM.',              'draft',     2),
    ('Web Performance',       'Optimizing web apps.',               'published', 3),
    ('Cargo Workspaces',      'Managing multi-crate projects.',     'published', 2),
    ('Type Safety',           'Leveraging the type system.',        'published', 2),
    ('Deploy with Docker',    'Containerizing Rust apps.',          'draft',     3),
    ('Testing Strategies',    'Unit and integration tests.',        'published', 1),
    ('Lifetimes Explained',   'Understanding borrow checker.',      'published', 2),
    ('REST API Design',       'Building clean REST APIs.',          'published', 3),
    ('Middleware in Axum',    'Writing custom middleware.',         'draft',     1),
    ('Database Migrations',   'Schema evolution strategies.',       'published', 2),
    ('Serde Deep Dive',       'Serialization and deserialization.', 'published', 1),
    ('CI/CD for Rust',        'GitHub Actions for Rust projects.',  'draft',     3),
    ('Benchmarking Rust',     'Using criterion.rs.',                'published', 2),
    ('Security Best Practices','Input validation and secrets.',     'published', 1),
    ('GraphQL with Rust',     'Building GraphQL APIs.',             'draft',     3),
    ('WASM and Rust',         'Compiling Rust to WebAssembly.',     'published', 1),
    ('Traits vs Generics',    'When to use each.',                  'published', 2),
    ('Macros in Rust',        'Procedural and declarative macros.', 'draft',     2)
    ON CONFLICT DO NOTHING;

INSERT INTO post_tags (post_id, tag_id) VALUES (1, 1), (1, 5), (2, 1), (3, 1), (4, 3), (5, 1)
    ON CONFLICT DO NOTHING;
