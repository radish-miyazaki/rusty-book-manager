INSERT INTO
    roles (name)
VALUES ('Admin'),
    ('User') ON CONFLICT DO NOTHING;

INSERT INTO
    users (
        name,
        email,
        password_hash,
        role_id
    )
SELECT 'Eleazar Fig', 'eleazar_fig@example.com', '$2b$12$sGXC.3Ew9yBl9dCKsQTfgebvkbkg/mRz9BRpL5fQgSU5TDDzta.Ay', role_id
FROM roles
WHERE
    name LIKE 'Admin';