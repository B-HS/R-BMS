SELECT 'chart_best rows whose assist_grade differs from the grade of the score they point to (assist-grade.backfill.sql would change)' AS target, COUNT(*) AS row_count
FROM `chart_best` AS `best`
INNER JOIN `score` ON `score`.`id` = `best`.`score_id`
WHERE `best`.`assist_grade` <> CASE WHEN `score`.`clear` = 3 THEN CASE `score`.`gauge` WHEN 1 THEN 1 WHEN 2 THEN 2 WHEN 3 THEN 3 WHEN 4 THEN 4 WHEN 5 THEN 4 WHEN 6 THEN 2 WHEN 7 THEN 3 WHEN 8 THEN 4 ELSE 0 END ELSE 0 END
UNION ALL SELECT 'chart_best rows with an assist_grade above 0', COUNT(*)
FROM `chart_best`
WHERE `assist_grade` > 0
UNION ALL SELECT 'chart_best rows whose score row is gone (the backfill can not reach them)', COUNT(*)
FROM `chart_best` AS `best`
WHERE NOT EXISTS (SELECT 1 FROM `score` WHERE `score`.`id` = `best`.`score_id`);
