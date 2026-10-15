CREATE TABLE `chart_notes_backup` (
    `sha256` char(64) NOT NULL PRIMARY KEY,
    `old_notes` int NOT NULL,
    `new_notes` int NOT NULL
);
INSERT INTO `chart_notes_backup` (`sha256`, `old_notes`, `new_notes`)
SELECT c.`sha256`, c.`notes`, agreed.`notes`
FROM `chart` c
JOIN (
    SELECT ranked.`chart_sha256`, ranked.`notes`
    FROM (
        SELECT
            s.`chart_sha256`,
            s.`notes`,
            ROW_NUMBER() OVER (PARTITION BY s.`chart_sha256` ORDER BY COUNT(DISTINCT s.`user_id`) DESC, COUNT(*) DESC, s.`notes` DESC) AS `pick`
        FROM `score` s
        WHERE s.`user_id` IS NOT NULL AND s.`lntype` = 0 AND s.`scratch_auto` = false AND s.`notes` > 0
        GROUP BY s.`chart_sha256`, s.`notes`
    ) ranked
    WHERE ranked.`pick` = 1
) agreed ON agreed.`chart_sha256` = c.`sha256`
WHERE c.`notes` <> agreed.`notes`;
UPDATE `chart` c
JOIN `chart_notes_backup` b ON b.`sha256` = c.`sha256`
SET c.`notes` = b.`new_notes`
WHERE c.`notes` = b.`old_notes`;
