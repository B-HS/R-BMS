SELECT 'chart rows whose notes differ from an accepted LN-mode total_notes' AS target, COUNT(*) AS row_count
FROM `chart` c
WHERE EXISTS (SELECT 1 FROM `score` s WHERE s.`chart_sha256` = c.`sha256` AND s.`lntype` = 0 AND s.`notes` > 0 AND s.`notes` <> c.`notes`)
UNION ALL SELECT 'chart rows chart-notes.repair.sql would change (to the LN-mode total_notes the most users sent)', COUNT(*)
FROM `chart` c
JOIN (
    SELECT ranked.`chart_sha256`, ranked.`notes`
    FROM (
        SELECT
            s.`chart_sha256`,
            s.`notes`,
            ROW_NUMBER() OVER (PARTITION BY s.`chart_sha256` ORDER BY COUNT(DISTINCT s.`user_id`) DESC, COUNT(*) DESC, s.`notes` DESC) AS `pick`
        FROM `score` s
        WHERE s.`user_id` IS NOT NULL AND s.`lntype` = 0 AND s.`notes` > 0
        GROUP BY s.`chart_sha256`, s.`notes`
    ) ranked
    WHERE ranked.`pick` = 1
) agreed ON agreed.`chart_sha256` = c.`sha256`
WHERE c.`notes` <> agreed.`notes`
UNION ALL SELECT 'chart rows that held a real play past twice their notes (would reject it before the fix)', COUNT(*)
FROM `chart` c
WHERE c.`notes` > 0 AND EXISTS (SELECT 1 FROM `score` s WHERE s.`chart_sha256` = c.`sha256` AND s.`notes` > c.`notes` * 2)
UNION ALL SELECT 'chart rows with no accepted score at all (made by POST /api/charts)', COUNT(*)
FROM `chart` c
WHERE NOT EXISTS (SELECT 1 FROM `score` s WHERE s.`chart_sha256` = c.`sha256`)
UNION ALL SELECT 'rejected submissions of any reason and time (the audit keeps neither chart nor reason)', COUNT(*)
FROM `submission_audit`
WHERE `accepted` = false
UNION ALL SELECT 'chart_notes_backup tables (1 = chart-notes.repair.sql has run, and it will not run again until that table is dropped)', COUNT(*)
FROM information_schema.`TABLES` t
WHERE t.`TABLE_SCHEMA` = DATABASE() AND t.`TABLE_NAME` = 'chart_notes_backup';
