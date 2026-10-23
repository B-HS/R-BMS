START TRANSACTION;

CREATE TEMPORARY TABLE `purge_chart` AS
SELECT `sha256` FROM `chart` WHERE `md5` = 'f8b3b8ca5b5c5f7be1352b7aae723bee'
UNION SELECT `chart_sha256` FROM `score` WHERE `chart_md5` = 'f8b3b8ca5b5c5f7be1352b7aae723bee'
UNION SELECT `chart_sha256` FROM `play_trace` WHERE `chart_md5` = 'f8b3b8ca5b5c5f7be1352b7aae723bee'
UNION SELECT `sha256` FROM `chart_pattern` WHERE `md5` = 'f8b3b8ca5b5c5f7be1352b7aae723bee';
CREATE TEMPORARY TABLE `purge_score` AS SELECT `id` FROM `score` WHERE `chart_sha256` IN (SELECT `sha256` FROM `purge_chart`);
CREATE TEMPORARY TABLE `purge_trace` AS SELECT `id` FROM `play_trace` WHERE `chart_sha256` IN (SELECT `sha256` FROM `purge_chart`) OR `score_id` IN (SELECT `id` FROM `purge_score`);

DELETE FROM `play_analysis` WHERE `trace_id` IN (SELECT `id` FROM `purge_trace`) OR `pattern_md5` = 'f8b3b8ca5b5c5f7be1352b7aae723bee';
DELETE FROM `play_trace` WHERE `id` IN (SELECT `id` FROM `purge_trace`);
DELETE FROM `replay` WHERE `chart_sha256` IN (SELECT `sha256` FROM `purge_chart`) OR `score_id` IN (SELECT `id` FROM `purge_score`);
DELETE FROM `chart_best` WHERE `chart_sha256` IN (SELECT `sha256` FROM `purge_chart`);
DELETE FROM `submission_audit` WHERE `score_id` IN (SELECT `id` FROM `purge_score`);
DELETE FROM `score` WHERE `id` IN (SELECT `id` FROM `purge_score`);
DELETE FROM `chart_pattern` WHERE `md5` = 'f8b3b8ca5b5c5f7be1352b7aae723bee' OR `sha256` IN (SELECT `sha256` FROM `purge_chart`);
DELETE FROM `table_chart` WHERE `chart_sha256` IN (SELECT `sha256` FROM `purge_chart`);
DELETE FROM `user_folder_item` WHERE `md5` = 'f8b3b8ca5b5c5f7be1352b7aae723bee' OR `sha256` IN (SELECT `sha256` FROM `purge_chart`);
DELETE FROM `ai_result` WHERE CAST(`output` AS CHAR CHARACTER SET utf8mb4) COLLATE utf8mb4_unicode_ci LIKE '%f8b3b8ca5b5c5f7be1352b7aae723bee%' OR CAST(`output` AS CHAR CHARACTER SET utf8mb4) COLLATE utf8mb4_unicode_ci LIKE '%架空ユリカゴ%' OR `subject_key` LIKE '%f8b3b8ca5b5c5f7be1352b7aae723bee%' OR EXISTS (SELECT 1 FROM `purge_chart` p WHERE CAST(`output` AS CHAR CHARACTER SET utf8mb4) COLLATE utf8mb4_unicode_ci LIKE CONCAT('%', p.`sha256`, '%') OR `subject_key` LIKE CONCAT('%', p.`sha256`, '%'));
DELETE FROM `chart` WHERE `sha256` IN (SELECT `sha256` FROM `purge_chart`) AND NOT EXISTS (SELECT 1 FROM `course_chart` cc WHERE cc.`chart_sha256` = `chart`.`sha256`);

DROP TEMPORARY TABLE `purge_trace`;
DROP TEMPORARY TABLE `purge_score`;
DROP TEMPORARY TABLE `purge_chart`;

COMMIT;
