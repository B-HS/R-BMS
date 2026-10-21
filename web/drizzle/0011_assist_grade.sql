ALTER TABLE `chart_best` ADD `assist_grade` tinyint DEFAULT 0 NOT NULL;--> statement-breakpoint
ALTER TABLE `course_best` ADD `assist_grade` tinyint DEFAULT 0 NOT NULL;--> statement-breakpoint
CREATE INDEX `chart_best_rank_grade_idx` ON `chart_best` (`chart_sha256`,`scratch_auto`,`clear`,`assist_grade`,`ex_score`);--> statement-breakpoint
UPDATE `chart_best` AS `best` INNER JOIN `score` ON `score`.`id` = `best`.`score_id` SET `best`.`assist_grade` = CASE WHEN `score`.`clear` = 3 THEN CASE `score`.`gauge` WHEN 1 THEN 1 WHEN 2 THEN 2 WHEN 3 THEN 3 WHEN 4 THEN 4 WHEN 5 THEN 4 WHEN 6 THEN 2 WHEN 7 THEN 3 WHEN 8 THEN 4 ELSE 0 END ELSE 0 END;
