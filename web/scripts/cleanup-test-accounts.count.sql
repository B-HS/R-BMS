SELECT `id`, `login_id`, `name`, `created_at`
FROM `user`
WHERE `login_id` IN ('e2e-phase-i-18d38e33cabb', 'e2e-phase-i-18d391944f88', 'gui-e2e-871795', 'probe1', 'w2b2probe');

SELECT 'user' AS target, COUNT(*) AS row_count FROM `user` WHERE `login_id` IN ('e2e-phase-i-18d38e33cabb', 'e2e-phase-i-18d391944f88', 'gui-e2e-871795', 'probe1', 'w2b2probe')
UNION ALL SELECT 'score', COUNT(*) FROM `score` WHERE `user_id` IN (SELECT `id` FROM `user` WHERE `login_id` IN ('e2e-phase-i-18d38e33cabb', 'e2e-phase-i-18d391944f88', 'gui-e2e-871795', 'probe1', 'w2b2probe'))
UNION ALL SELECT 'chart_best', COUNT(*) FROM `chart_best` WHERE `user_id` IN (SELECT `id` FROM `user` WHERE `login_id` IN ('e2e-phase-i-18d38e33cabb', 'e2e-phase-i-18d391944f88', 'gui-e2e-871795', 'probe1', 'w2b2probe'))
UNION ALL SELECT 'replay', COUNT(*) FROM `replay` WHERE `user_id` IN (SELECT `id` FROM `user` WHERE `login_id` IN ('e2e-phase-i-18d38e33cabb', 'e2e-phase-i-18d391944f88', 'gui-e2e-871795', 'probe1', 'w2b2probe'))
    OR `score_id` IN (SELECT `id` FROM `score` WHERE `user_id` IN (SELECT `id` FROM `user` WHERE `login_id` IN ('e2e-phase-i-18d38e33cabb', 'e2e-phase-i-18d391944f88', 'gui-e2e-871795', 'probe1', 'w2b2probe')))
UNION ALL SELECT 'course_score', COUNT(*) FROM `course_score` WHERE `user_id` IN (SELECT `id` FROM `user` WHERE `login_id` IN ('e2e-phase-i-18d38e33cabb', 'e2e-phase-i-18d391944f88', 'gui-e2e-871795', 'probe1', 'w2b2probe'))
UNION ALL SELECT 'course_best', COUNT(*) FROM `course_best` WHERE `user_id` IN (SELECT `id` FROM `user` WHERE `login_id` IN ('e2e-phase-i-18d38e33cabb', 'e2e-phase-i-18d391944f88', 'gui-e2e-871795', 'probe1', 'w2b2probe'))
UNION ALL SELECT 'submission_audit', COUNT(*) FROM `submission_audit` WHERE `user_id` IN (SELECT `id` FROM `user` WHERE `login_id` IN ('e2e-phase-i-18d38e33cabb', 'e2e-phase-i-18d391944f88', 'gui-e2e-871795', 'probe1', 'w2b2probe'))
UNION ALL SELECT 'rival', COUNT(*) FROM `rival` WHERE `user_id` IN (SELECT `id` FROM `user` WHERE `login_id` IN ('e2e-phase-i-18d38e33cabb', 'e2e-phase-i-18d391944f88', 'gui-e2e-871795', 'probe1', 'w2b2probe'))
    OR `rival_id` IN (SELECT `id` FROM `user` WHERE `login_id` IN ('e2e-phase-i-18d38e33cabb', 'e2e-phase-i-18d391944f88', 'gui-e2e-871795', 'probe1', 'w2b2probe'))
UNION ALL SELECT 'setting_blob', COUNT(*) FROM `setting_blob` WHERE `user_id` IN (SELECT `id` FROM `user` WHERE `login_id` IN ('e2e-phase-i-18d38e33cabb', 'e2e-phase-i-18d391944f88', 'gui-e2e-871795', 'probe1', 'w2b2probe'))
UNION ALL SELECT 'api_token', COUNT(*) FROM `api_token` WHERE `user_id` IN (SELECT `id` FROM `user` WHERE `login_id` IN ('e2e-phase-i-18d38e33cabb', 'e2e-phase-i-18d391944f88', 'gui-e2e-871795', 'probe1', 'w2b2probe'))
UNION ALL SELECT 'session', COUNT(*) FROM `session` WHERE `user_id` IN (SELECT `id` FROM `user` WHERE `login_id` IN ('e2e-phase-i-18d38e33cabb', 'e2e-phase-i-18d391944f88', 'gui-e2e-871795', 'probe1', 'w2b2probe'))
UNION ALL SELECT 'account', COUNT(*) FROM `account` WHERE `user_id` IN (SELECT `id` FROM `user` WHERE `login_id` IN ('e2e-phase-i-18d38e33cabb', 'e2e-phase-i-18d391944f88', 'gui-e2e-871795', 'probe1', 'w2b2probe'))
UNION ALL SELECT 'user_folder', COUNT(*) FROM `user_folder` WHERE `user_id` IN (SELECT `id` FROM `user` WHERE `login_id` IN ('e2e-phase-i-18d38e33cabb', 'e2e-phase-i-18d391944f88', 'gui-e2e-871795', 'probe1', 'w2b2probe'))
UNION ALL SELECT 'user_folder_item', COUNT(*) FROM `user_folder_item` WHERE `folder_id` IN (SELECT `id` FROM `user_folder` WHERE `user_id` IN (SELECT `id` FROM `user` WHERE `login_id` IN ('e2e-phase-i-18d38e33cabb', 'e2e-phase-i-18d391944f88', 'gui-e2e-871795', 'probe1', 'w2b2probe')))
UNION ALL SELECT 'score_other_users', COUNT(*) FROM `score` WHERE `user_id` IS NULL OR `user_id` NOT IN (SELECT `id` FROM `user` WHERE `login_id` IN ('e2e-phase-i-18d38e33cabb', 'e2e-phase-i-18d391944f88', 'gui-e2e-871795', 'probe1', 'w2b2probe'));
