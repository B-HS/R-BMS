START TRANSACTION;

CREATE TEMPORARY TABLE `cleanup_test_user` AS SELECT `id` FROM `user` WHERE `login_id` IN ('e2e-phase-i-18d38e33cabb', 'e2e-phase-i-18d391944f88', 'gui-e2e-871795', 'probe1', 'w2b2probe');
CREATE TEMPORARY TABLE `cleanup_test_score` AS SELECT `id` FROM `score` WHERE `user_id` IN (SELECT `id` FROM `cleanup_test_user`);

DELETE FROM `replay` WHERE `score_id` IN (SELECT `id` FROM `cleanup_test_score`);
DELETE FROM `replay` WHERE `user_id` IN (SELECT `id` FROM `cleanup_test_user`);
DELETE FROM `chart_best` WHERE `user_id` IN (SELECT `id` FROM `cleanup_test_user`);
DELETE FROM `submission_audit` WHERE `user_id` IN (SELECT `id` FROM `cleanup_test_user`);
DELETE FROM `score` WHERE `id` IN (SELECT `id` FROM `cleanup_test_score`);
DELETE FROM `course_best` WHERE `user_id` IN (SELECT `id` FROM `cleanup_test_user`);
DELETE FROM `course_score` WHERE `user_id` IN (SELECT `id` FROM `cleanup_test_user`);
DELETE FROM `rival` WHERE `user_id` IN (SELECT `id` FROM `cleanup_test_user`);
DELETE FROM `rival` WHERE `rival_id` IN (SELECT `id` FROM `cleanup_test_user`);
DELETE FROM `setting_blob` WHERE `user_id` IN (SELECT `id` FROM `cleanup_test_user`);
DELETE FROM `user` WHERE `id` IN (SELECT `id` FROM `cleanup_test_user`);

SELECT 'user_left' AS target, COUNT(*) AS row_count FROM `user` WHERE `login_id` IN ('e2e-phase-i-18d38e33cabb', 'e2e-phase-i-18d391944f88', 'gui-e2e-871795', 'probe1', 'w2b2probe')
UNION ALL SELECT 'session_left', COUNT(*) FROM `session` WHERE `user_id` IN (SELECT `id` FROM `cleanup_test_user`);

DROP TEMPORARY TABLE `cleanup_test_score`;
DROP TEMPORARY TABLE `cleanup_test_user`;

COMMIT;
