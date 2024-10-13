INSERT INTO roles (name) VALUES ('Admin'), ('User');

INSERT INTO users (user_id, name, email, password_hash, role_id)
SELECT
    '2bbd820c-7a88-450c-b056-19dcbadd527d'
    , 'Eleazar Fig'
    , 'eleazar.fig@example.com'
    , '$2b$12$sGXC.3Ew9yBl9dCKsQTfgebvkbkg/mRz9BRpL5fQgSU5TDDzta.Ay'
    , role_id
FROM roles WHERE name = 'Admin';
