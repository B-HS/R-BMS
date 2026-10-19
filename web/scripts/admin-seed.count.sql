SELECT 'user_with_email' AS target, COUNT(*) AS row_count FROM `user` WHERE `email` = 'hs@gumyo.net'
UNION ALL SELECT 'user_to_change', COUNT(*) FROM `user` WHERE `email` = 'hs@gumyo.net' AND `role` <> 'admin'
UNION ALL SELECT 'admins', COUNT(*) FROM `user` WHERE `role` = 'admin';
