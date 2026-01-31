-- User search query (frequently used)
SELECT u.*, COUNT(o.id) as order_count
FROM users u
LEFT JOIN orders o ON o.user_id = u.id
WHERE u.email LIKE '%@example.com'
  AND u.created_at > '2024-01-01'
  AND u.status = 'active'
GROUP BY u.id
ORDER BY u.last_login DESC
LIMIT 100;

-- Missing indexes on: email, created_at, status, last_login
